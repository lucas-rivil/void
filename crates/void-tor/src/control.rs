use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::{sleep, Instant};

use crate::{TorError, TorResult};

pub struct ControlClient {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: tokio::net::tcp::OwnedWriteHalf,
}

impl ControlClient {
    pub async fn connect_authenticated(addr: SocketAddr, cookie_file: &Path) -> TorResult<Self> {
        let stream = connect_with_retry(addr).await?;
        let cookie = read_cookie_with_retry(cookie_file).await?;
        let (reader, writer) = stream.into_split();
        let mut client = Self {
            reader: BufReader::new(reader),
            writer,
        };
        let hex_cookie = hex::encode(&cookie);
        client.send(&format!("AUTHENTICATE {hex_cookie}")).await?;
        Ok(client)
    }

    pub async fn send(&mut self, command: &str) -> TorResult<Vec<String>> {
        self.writer.write_all(command.as_bytes()).await?;
        self.writer.write_all(b"\r\n").await?;
        self.writer.flush().await?;
        self.read_reply().await
    }

    pub async fn get_info(&mut self, key: &str) -> TorResult<String> {
        let lines = self.send(&format!("GETINFO {key}")).await?;
        let prefix = format!("{key}=");
        for line in &lines {
            if let Some(value) = line.strip_prefix(&prefix) {
                return Ok(value.trim_matches('"').to_string());
            }
        }
        Ok(String::new())
    }

    pub async fn add_onion(
        &mut self,
        key_b64: &str,
        virtual_port: u16,
        target_port: u16,
    ) -> TorResult<String> {
        let lines = self
            .send(&format!(
                "ADD_ONION ED25519-V3:{key_b64} Flags=Detach Port={virtual_port},127.0.0.1:{target_port}"
            ))
            .await?;
        for line in &lines {
            if let Some(id) = line.strip_prefix("ServiceID=") {
                return Ok(id.trim().to_string());
            }
        }
        Err(TorError::Control(
            "ServiceID absent de la réponse ADD_ONION".into(),
        ))
    }

    async fn read_reply(&mut self) -> TorResult<Vec<String>> {
        let mut lines = Vec::new();
        let mut line = String::new();
        loop {
            line.clear();
            let n = self.reader.read_line(&mut line).await?;
            if n == 0 {
                return Err(TorError::Control(
                    "connexion control fermée par tor".into(),
                ));
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            match parse_line(trimmed) {
                Some((code, is_final, rest)) => {
                    lines.push(rest.to_string());
                    if is_final {
                        return if (200..300).contains(&code) {
                            Ok(lines)
                        } else {
                            Err(TorError::Control(format!(
                                "erreur tor {code}: {}",
                                lines.join(" / ")
                            )))
                        };
                    }
                }
                None => continue,
            }
        }
    }
}

async fn connect_with_retry(addr: SocketAddr) -> TorResult<TcpStream> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match TcpStream::connect(addr).await {
            Ok(stream) => return Ok(stream),
            Err(_) if Instant::now() < deadline => sleep(Duration::from_millis(250)).await,
            Err(e) => return Err(e.into()),
        }
    }
}

async fn read_cookie_with_retry(cookie_file: &Path) -> TorResult<Vec<u8>> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match std::fs::read(cookie_file) {
            Ok(cookie) if !cookie.is_empty() => return Ok(cookie),
            _ if Instant::now() < deadline => sleep(Duration::from_millis(250)).await,
            _ => {
                return Err(TorError::Timeout(
                    "fichier cookie control".into(),
                ))
            }
        }
    }
}

fn parse_line(line: &str) -> Option<(u16, bool, &str)> {
    let bytes = line.as_bytes();
    if bytes.len() < 3 || !bytes[..3].iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let code: u16 = line[..3].parse().ok()?;
    if bytes.len() == 3 {
        return Some((code, true, ""));
    }
    let sep = bytes[3];
    let rest = &line[4.min(line.len())..];
    let is_final = sep != b'-' && sep != b'+';
    Some((code, is_final, rest))
}

#[cfg(test)]
mod tests {
    use super::parse_line;

    #[test]
    fn parse_continuation() {
        let (code, fin, rest) = parse_line("250-ServiceID=abc").unwrap();
        assert_eq!((code, fin), (250, false));
        assert_eq!(rest, "ServiceID=abc");
    }

    #[test]
    fn parse_final_ok() {
        let (code, fin, rest) = parse_line("250 OK").unwrap();
        assert_eq!((code, fin), (250, true));
        assert_eq!(rest, "OK");
    }

    #[test]
    fn parse_final_err() {
        let (code, fin, _) = parse_line("512 Unrecognized command").unwrap();
        assert_eq!((code, fin), (512, true));
    }

    #[test]
    fn parse_bare_code() {
        let (code, fin, rest) = parse_line("250").unwrap();
        assert_eq!((code, fin, rest), (250, true, ""));
    }

    #[test]
    fn parse_junk() {
        assert!(parse_line("Bootstrap progress 45%").is_none());
    }
}
