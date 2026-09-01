use void_crypto::{onion_id_to_public, Identity};

pub const HELLO_DOMAIN: &[u8] = b"void-handshake-hello-v1";
pub const WELCOME_DOMAIN: &[u8] = b"void-handshake-welcome-v1";

pub fn new_nonce() -> [u8; 16] {
    use rand_core::RngCore;
    let mut nonce = [0u8; 16];
    rand_core::OsRng.fill_bytes(&mut nonce);
    nonce
}

pub fn handshake_message(domain: &[u8], nonce: &[u8; 16], onion_id: &str) -> Vec<u8> {
    let mut message = Vec::with_capacity(domain.len() + nonce.len() + onion_id.len());
    message.extend_from_slice(domain);
    message.extend_from_slice(nonce);
    message.extend_from_slice(onion_id.as_bytes());
    message
}

pub fn sign_handshake(
    identity: &Identity,
    domain: &[u8],
    nonce: &[u8; 16],
) -> [u8; 64] {
    let message = handshake_message(domain, nonce, &identity.onion_id());
    identity.sign_bytes(&message)
}

pub fn verify_handshake(
    domain: &[u8],
    onion_id: &str,
    nonce: &[u8; 16],
    signature: &[u8; 64],
) -> bool {
    let Some(public) = onion_id_to_public(onion_id) else {
        return false;
    };
    let message = handshake_message(domain, nonce, onion_id);
    Identity::verify_public_bytes(&public, &message, signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_sign_verify() {
        let identity = Identity::generate();
        let nonce = new_nonce();
        let signature = sign_handshake(&identity, HELLO_DOMAIN, &nonce);
        assert!(verify_handshake(
            HELLO_DOMAIN,
            &identity.onion_id(),
            &nonce,
            &signature
        ));
    }

    #[test]
    fn handshake_reject_wrong_domain() {
        let identity = Identity::generate();
        let nonce = new_nonce();
        let signature = sign_handshake(&identity, HELLO_DOMAIN, &nonce);
        assert!(!verify_handshake(
            WELCOME_DOMAIN,
            &identity.onion_id(),
            &nonce,
            &signature
        ));
    }

    #[test]
    fn handshake_reject_tampered_nonce() {
        let identity = Identity::generate();
        let nonce = new_nonce();
        let signature = sign_handshake(&identity, HELLO_DOMAIN, &nonce);
        let mut other = nonce;
        other[0] ^= 1;
        assert!(!verify_handshake(
            HELLO_DOMAIN,
            &identity.onion_id(),
            &other,
            &signature
        ));
    }

    #[test]
    fn handshake_reject_foreign_onion() {
        let identity = Identity::generate();
        let other = Identity::generate();
        let nonce = new_nonce();
        let signature = sign_handshake(&identity, HELLO_DOMAIN, &nonce);
        assert!(!verify_handshake(
            HELLO_DOMAIN,
            &other.onion_id(),
            &nonce,
            &signature
        ));
    }

    #[test]
    fn handshake_reject_invalid_onion() {
        let identity = Identity::generate();
        let nonce = new_nonce();
        let signature = sign_handshake(&identity, HELLO_DOMAIN, &nonce);
        assert!(!verify_handshake(HELLO_DOMAIN, "abc", &nonce, &signature));
    }
}
