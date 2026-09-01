use anyhow::Result;
use rusqlite::{params, Connection};

pub const STORE_SCHEMA_VERSION: u32 = 3;

fn migrate_messages_columns(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(messages)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<_, _>>()?;
    if !columns.iter().any(|c| c == "kind") {
        conn.execute("ALTER TABLE messages ADD COLUMN kind INTEGER NOT NULL DEFAULT 0", [])?;
    }
    if !columns.iter().any(|c| c == "duration_ms") {
        conn.execute(
            "ALTER TABLE messages ADD COLUMN duration_ms INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
}

pub struct Store {
    conn: Connection,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DmRecord {
    pub id: String,
    pub peer_id: String,
    pub author_id: String,
    pub body: String,
    pub created_ms: u64,
    pub status: u8,
    pub kind: u8,
    pub duration_ms: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelayRow {
    pub id: [u8; 16],
    pub sender_id: String,
    pub recipient_id: String,
    pub kind: u8,
    pub payload: Vec<u8>,
    pub stored_ms: u64,
}

impl Store {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                peer_id TEXT NOT NULL,
                author_id TEXT NOT NULL,
                body TEXT NOT NULL,
                created_ms INTEGER NOT NULL,
                status INTEGER NOT NULL DEFAULT 0,
                kind INTEGER NOT NULL DEFAULT 0,
                duration_ms INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_messages_peer
                ON messages(peer_id, id);
            CREATE TABLE IF NOT EXISTS relay_queue (
                id BLOB PRIMARY KEY,
                sender_id TEXT NOT NULL,
                recipient_id TEXT NOT NULL,
                kind INTEGER NOT NULL,
                payload BLOB NOT NULL,
                stored_ms INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_relay_recipient
                ON relay_queue(recipient_id, stored_ms);",
        )?;
        migrate_messages_columns(&conn)?;
        conn.pragma_update(None, "user_version", STORE_SCHEMA_VERSION as i64)?;
        Ok(Self { conn })
    }

    pub fn insert_message(&mut self, record: &DmRecord) -> Result<bool> {
        let changed = self.conn.execute(
            "INSERT OR IGNORE INTO messages (id, peer_id, author_id, body, created_ms, status, kind, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id,
                record.peer_id,
                record.author_id,
                record.body,
                record.created_ms as i64,
                record.status as i64,
                record.kind as i64,
                record.duration_ms as i64
            ],
        )?;
        Ok(changed > 0)
    }

    pub fn list_messages(
        &mut self,
        peer_id: &str,
        limit: u64,
        before_id: Option<&str>,
    ) -> Result<Vec<DmRecord>> {
        let limit = limit.clamp(1, 500);
        let mut stmt = self.conn.prepare(
            "SELECT id, peer_id, author_id, body, created_ms, status, kind, duration_ms
             FROM messages
             WHERE peer_id = ?1 AND (?2 IS NULL OR id < ?2)
             ORDER BY id DESC
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![peer_id, before_id, limit], map_row)?;
        let mut out: Vec<DmRecord> = rows.collect::<std::result::Result<_, _>>()?;
        out.reverse();
        Ok(out)
    }

    pub fn set_status(&mut self, id: &str, status: u8) -> Result<bool> {
        let changed = self.conn.execute(
            "UPDATE messages SET status = ?2 WHERE id = ?1 AND status < ?2",
            params![id, status as i64],
        )?;
        Ok(changed > 0)
    }

    pub fn delete_conversation(&mut self, peer_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM messages WHERE peer_id = ?1", params![peer_id])?;
        Ok(())
    }

    pub fn queued_messages(&mut self, author_id: &str) -> Result<Vec<DmRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, peer_id, author_id, body, created_ms, status, kind, duration_ms
             FROM messages
             WHERE author_id = ?1 AND status = 0
             ORDER BY id
             LIMIT 500",
        )?;
        let rows = stmt.query_map(params![author_id], map_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn messages_from_since(
        &mut self,
        peer_id: &str,
        author_id: &str,
        after_ms: u64,
        limit: u64,
    ) -> Result<Vec<DmRecord>> {
        let limit = limit.clamp(1, 500);
        let mut stmt = self.conn.prepare(
            "SELECT id, peer_id, author_id, body, created_ms, status, kind, duration_ms
             FROM messages
             WHERE peer_id = ?1 AND author_id = ?2 AND created_ms > ?3
             ORDER BY id
             LIMIT ?4",
        )?;
        let rows = stmt.query_map(params![peer_id, author_id, after_ms as i64, limit], map_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn max_created_ms(&mut self, peer_id: &str, author_id: &str) -> Result<Option<u64>> {
        let mut stmt = self.conn.prepare(
            "SELECT MAX(created_ms) FROM messages WHERE peer_id = ?1 AND author_id = ?2",
        )?;
        let value = stmt.query_row(params![peer_id, author_id], |row| {
            row.get::<_, Option<i64>>(0)
        })?;
        Ok(value.map(|v| v as u64))
    }

    pub fn relay_insert(&mut self, row: &RelayRow) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO relay_queue (id, sender_id, recipient_id, kind, payload, stored_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                row.id,
                row.sender_id,
                row.recipient_id,
                row.kind as i64,
                row.payload,
                row.stored_ms as i64
            ],
        )?;
        Ok(())
    }

    pub fn relay_for_recipient(&mut self, recipient_id: &str) -> Result<Vec<RelayRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sender_id, recipient_id, kind, payload, stored_ms
             FROM relay_queue
             WHERE recipient_id = ?1
             ORDER BY stored_ms
             LIMIT 200",
        )?;
        let rows = stmt.query_map(params![recipient_id], |row| {
            Ok(RelayRow {
                id: row.get::<_, Vec<u8>>(0)?.try_into().unwrap_or([0; 16]),
                sender_id: row.get(1)?,
                recipient_id: row.get(2)?,
                kind: row.get::<_, i64>(3)? as u8,
                payload: row.get(4)?,
                stored_ms: row.get::<_, i64>(5)? as u64,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn relay_delete_ids(&mut self, ids: &[[u8; 16]]) -> Result<usize> {
        let mut count = 0;
        for id in ids {
            count += self.conn.execute(
                "DELETE FROM relay_queue WHERE id = ?1",
                params![id],
            )?;
        }
        Ok(count)
    }

    pub fn relay_purge_expired(&mut self, before_ms: u64) -> Result<usize> {
        let changed = self.conn.execute(
            "DELETE FROM relay_queue WHERE stored_ms < ?1",
            params![before_ms as i64],
        )?;
        Ok(changed)
    }

    pub fn relay_count(&mut self) -> Result<u64> {
        let count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM relay_queue", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    pub fn relay_count_from(&mut self, sender_id: &str) -> Result<u64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM relay_queue WHERE sender_id = ?1",
            params![sender_id],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DmRecord> {
    Ok(DmRecord {
        id: row.get(0)?,
        peer_id: row.get(1)?,
        author_id: row.get(2)?,
        body: row.get(3)?,
        created_ms: row.get::<_, i64>(4)? as u64,
        status: row.get::<_, i64>(5)? as u8,
        kind: row.get::<_, i64>(6)? as u8,
        duration_ms: row.get::<_, i64>(7)? as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) -> Store {
        let path = std::env::temp_dir().join(format!(
            "void-store-test-{tag}-{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        Store::open(&path).unwrap()
    }

    fn record(id: &str, peer: &str, author: &str, body: &str) -> DmRecord {
        DmRecord {
            id: id.to_string(),
            peer_id: peer.to_string(),
            author_id: author.to_string(),
            body: body.to_string(),
            created_ms: 1000,
            status: 1,
            kind: 0,
            duration_ms: 0,
        }
    }

    #[test]
    fn insert_and_list() {
        let mut store = temp_store("list");
        let peer = "peer".to_string();
        for i in 0..5 {
            let id = format!("01HZZZZZZZZZZZZZZZZZZZZZZ{:02}", i);
            store
                .insert_message(&record(&id, &peer, "me", &format!("m{i}")))
                .unwrap();
        }
        let list = store.list_messages(&peer, 50, None).unwrap();
        assert_eq!(list.len(), 5);
        assert_eq!(list[0].body, "m0");
        assert_eq!(list[4].body, "m4");
    }

    #[test]
    fn insert_or_ignore_dedupe() {
        let mut store = temp_store("dedupe");
        let rec = record("01HZZZZZZZZZZZZZZZZZZZZZC1", "p", "me", "x");
        assert!(store.insert_message(&rec).unwrap());
        assert!(!store.insert_message(&rec).unwrap());
        assert_eq!(store.list_messages("p", 10, None).unwrap().len(), 1);
    }

    #[test]
    fn pagination_before_id() {
        let mut store = temp_store("page");
        let peer = "peer".to_string();
        for i in 0..5 {
            let id = format!("01HZZZZZZZZZZZZZZZZZZZZZZ{:02}", i);
            store
                .insert_message(&record(&id, &peer, "me", &format!("m{i}")))
                .unwrap();
        }
        let first_page = store.list_messages(&peer, 2, None).unwrap();
        assert_eq!(first_page.len(), 2);
        assert_eq!(first_page[0].body, "m3");
        assert_eq!(first_page[1].body, "m4");
        let second_page = store
            .list_messages(&peer, 2, Some(&first_page[0].id))
            .unwrap();
        assert_eq!(second_page.len(), 2);
        assert_eq!(second_page[0].body, "m1");
        assert_eq!(second_page[1].body, "m2");
    }

    #[test]
    fn status_monotonic() {
        let mut store = temp_store("status");
        let mut rec = record("01HZZZZZZZZZZZZZZZZZZZZZZ99", "p", "me", "x");
        rec.status = 0;
        store.insert_message(&rec).unwrap();
        assert!(store.set_status(&rec.id, 1).unwrap());
        assert!(store.set_status(&rec.id, 2).unwrap());
        assert!(!store.set_status(&rec.id, 1).unwrap());
        let list = store.list_messages("p", 10, None).unwrap();
        assert_eq!(list[0].status, 2);
    }

    #[test]
    fn peer_isolation() {
        let mut store = temp_store("iso");
        store
            .insert_message(&record("01HZZZZZZZZZZZZZZZZZZZZZA1", "p1", "me", "a"))
            .unwrap();
        store
            .insert_message(&record("01HZZZZZZZZZZZZZZZZZZZZZA2", "p2", "me", "b"))
            .unwrap();
        assert_eq!(store.list_messages("p1", 10, None).unwrap().len(), 1);
        assert_eq!(store.list_messages("p2", 10, None).unwrap().len(), 1);
    }

    #[test]
    fn delete_conversation() {
        let mut store = temp_store("del");
        store
            .insert_message(&record("01HZZZZZZZZZZZZZZZZZZZZZB1", "p1", "me", "a"))
            .unwrap();
        store
            .insert_message(&record("01HZZZZZZZZZZZZZZZZZZZZZB2", "p1", "me", "b"))
            .unwrap();
        store
            .insert_message(&record("01HZZZZZZZZZZZZZZZZZZZZZB3", "p2", "me", "c"))
            .unwrap();
        store.delete_conversation("p1").unwrap();
        assert_eq!(store.list_messages("p1", 10, None).unwrap().len(), 0);
        assert_eq!(store.list_messages("p2", 10, None).unwrap().len(), 1);
    }

    #[test]
    fn queued_and_since() {
        let mut store = temp_store("queued");
        let mut queued = record("01HZZZZZZZZZZZZZZZZZZZZZD1", "p1", "me", "à envoyer");
        queued.status = 0;
        store.insert_message(&queued).unwrap();
        store
            .insert_message(&record("01HZZZZZZZZZZZZZZZZZZZZZD2", "p1", "me", "envoyé"))
            .unwrap();
        let mut sent = record("01HZZZZZZZZZZZZZZZZZZZZZD3", "p1", "peer", "reçu");
        sent.created_ms = 2000;
        store.insert_message(&sent).unwrap();

        let queued_list = store.queued_messages("me").unwrap();
        assert_eq!(queued_list.len(), 1);
        assert_eq!(queued_list[0].body, "à envoyer");

        let mut late = record("01HZZZZZZZZZZZZZZZZZZZZZD4", "p1", "me", "tardif");
        late.created_ms = 3000;
        store.insert_message(&late).unwrap();
        let since = store.messages_from_since("p1", "me", 1000, 50).unwrap();
        assert_eq!(since.len(), 1);
        assert_eq!(since[0].body, "tardif");

        assert_eq!(store.max_created_ms("p1", "peer").unwrap(), Some(2000));
        assert_eq!(store.max_created_ms("p1", "inconnu").unwrap(), None);
    }

    #[test]
    fn voice_columns_and_migration() {
        let path = std::env::temp_dir().join(format!(
            "void-store-test-voice-{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE messages (
                    id TEXT PRIMARY KEY,
                    peer_id TEXT NOT NULL,
                    author_id TEXT NOT NULL,
                    body TEXT NOT NULL,
                    created_ms INTEGER NOT NULL,
                    status INTEGER NOT NULL DEFAULT 0
                );",
            )
            .unwrap();
        }
        let mut store = Store::open(&path).unwrap();
        let mut voice = record("01HZZZZZZZZZZZZZZZZZZZZZV1", "p", "me", "");
        voice.kind = 1;
        voice.duration_ms = 3500;
        assert!(store.insert_message(&voice).unwrap());
        let list = store.list_messages("p", 10, None).unwrap();
        assert_eq!(list[0].kind, 1);
        assert_eq!(list[0].duration_ms, 3500);
        let text = record("01HZZZZZZZZZZZZZZZZZZZZZV2", "p", "me", "salut");
        assert!(store.insert_message(&text).unwrap());
        let list = store.list_messages("p", 10, None).unwrap();
        assert_eq!(list[1].kind, 0);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn relay_queue_lifecycle() {
        let mut store = temp_store("relay");
        let row = |id: u8, recipient: &str, stored_ms: u64| RelayRow {
            id: [id; 16],
            sender_id: "a".repeat(56),
            recipient_id: recipient.to_string(),
            kind: 1,
            payload: vec![1, 2, 3],
            stored_ms,
        };
        store.relay_insert(&row(1, "b", 100)).unwrap();
        store.relay_insert(&row(2, "b", 200)).unwrap();
        store.relay_insert(&row(3, "c", 300)).unwrap();
        assert_eq!(store.relay_count().unwrap(), 3);
        assert_eq!(store.relay_count_from(&"a".repeat(56)).unwrap(), 3);

        let for_b = store.relay_for_recipient("b").unwrap();
        assert_eq!(for_b.len(), 2);
        assert_eq!(for_b[0].stored_ms, 100);

        store.relay_delete_ids(&[[1; 16]]).unwrap();
        assert_eq!(store.relay_count().unwrap(), 2);

        store.relay_purge_expired(250).unwrap();
        assert_eq!(store.relay_count().unwrap(), 1);
        assert_eq!(store.relay_for_recipient("b").unwrap().len(), 0);
        assert_eq!(store.relay_for_recipient("c").unwrap().len(), 1);
    }
}
