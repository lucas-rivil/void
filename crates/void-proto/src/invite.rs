use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use void_crypto::onion_id_is_valid;

pub const INVITE_SCHEME: &str = "void://invite";
pub const MAX_DISPLAY_NAME: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum InviteError {
    #[error("lien d'invitation invalide: {0}")]
    Invalid(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Invite {
    pub onion_id: String,
    pub fingerprint: String,
    pub display_name: String,
}

impl Invite {
    pub fn new(
        onion_id: impl Into<String>,
        fingerprint: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            onion_id: onion_id.into(),
            fingerprint: fingerprint.into(),
            display_name: display_name.into(),
        }
    }

    pub fn to_link(&self) -> String {
        let name = utf8_percent_encode(&self.display_name, NON_ALPHANUMERIC);
        format!(
            "{INVITE_SCHEME}?onion={}&fp={}&n={}",
            self.onion_id, self.fingerprint, name
        )
    }

    pub fn parse(link: &str) -> Result<Self, InviteError> {
        let trimmed = link.trim();
        let rest = trimmed
            .strip_prefix(INVITE_SCHEME)
            .ok_or_else(|| InviteError::Invalid("le lien doit commencer par void://invite".into()))?;
        let query = rest
            .strip_prefix('?')
            .ok_or_else(|| InviteError::Invalid("paramètres manquants".into()))?;

        let mut onion_id: Option<String> = None;
        let mut fingerprint: Option<String> = None;
        let mut display_name = String::new();

        for pair in query.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            match key {
                "onion" => onion_id = Some(value.to_string()),
                "fp" => fingerprint = Some(value.to_string()),
                "n" => {
                    display_name = percent_decode_str(value)
                        .decode_utf8()
                        .map_err(|_| InviteError::Invalid("nom illisible".into()))?
                        .into_owned();
                }
                _ => {}
            }
        }

        let onion_id = onion_id
            .ok_or_else(|| InviteError::Invalid("paramètre onion manquant".into()))?;
        if !onion_id_is_valid(&onion_id) {
            return Err(InviteError::Invalid("adresse oignon invalide".into()));
        }
        let fingerprint = fingerprint
            .ok_or_else(|| InviteError::Invalid("paramètre fp manquant".into()))?;
        if fingerprint.is_empty()
            || fingerprint.len() > 64
            || !fingerprint.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(InviteError::Invalid("empreinte invalide".into()));
        }
        if display_name.chars().count() > MAX_DISPLAY_NAME {
            return Err(InviteError::Invalid("nom trop long".into()));
        }

        Ok(Self {
            onion_id,
            fingerprint: fingerprint.to_lowercase(),
            display_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use void_crypto::Identity;

    fn sample() -> (Identity, Invite) {
        let identity = Identity::generate();
        let invite = Invite::new(
            identity.onion_id(),
            identity.fingerprint_short(),
            "Zoé ✨",
        );
        (identity, invite)
    }

    #[test]
    fn roundtrip() {
        let (_, invite) = sample();
        let link = invite.to_link();
        assert!(link.starts_with("void://invite?onion="));
        assert_eq!(Invite::parse(&link).unwrap(), invite);
    }

    #[test]
    fn parse_strips_whitespace() {
        let (_, invite) = sample();
        let link = format!("  {}  ", invite.to_link());
        assert_eq!(Invite::parse(&link).unwrap(), invite);
    }

    #[test]
    fn reject_wrong_scheme() {
        assert!(Invite::parse("https://void.example/invite?onion=abc").is_err());
    }

    #[test]
    fn reject_missing_onion() {
        assert!(Invite::parse("void://invite?fp=ab&n=x").is_err());
    }

    #[test]
    fn reject_corrupted_onion() {
        let (identity, _) = sample();
        let mut id = identity.onion_id();
        let last = id.pop().unwrap();
        id.push(if last == 'a' { 'b' } else { 'a' });
        let link = format!("void://invite?onion={id}&fp=ab12");
        assert!(Invite::parse(&link).is_err());
    }

    #[test]
    fn reject_bad_fingerprint() {
        let (identity, _) = sample();
        let link = format!("void://invite?onion={}&fp=xyz!", identity.onion_id());
        assert!(Invite::parse(&link).is_err());
    }

    #[test]
    fn reject_long_name() {
        let (identity, _) = sample();
        let name = "x".repeat(33);
        let link = format!(
            "void://invite?onion={}&fp=ab&n={name}",
            identity.onion_id()
        );
        assert!(Invite::parse(&link).is_err());
    }

    #[test]
    fn accept_empty_name() {
        let (identity, _) = sample();
        let link = format!("void://invite?onion={}&fp=ab", identity.onion_id());
        let parsed = Invite::parse(&link).unwrap();
        assert_eq!(parsed.display_name, "");
    }
}
