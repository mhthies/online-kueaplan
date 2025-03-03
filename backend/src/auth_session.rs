use crate::data_store::PassphraseId;
use base64::{DecodeError, Engine};
use ring::hmac::Key;

static HMAC_ALGORITHM: ring::hmac::Algorithm = ring::hmac::HMAC_SHA256;
const KEY_LENGTH: usize = 512 / 8;

#[derive(Debug)]
pub struct SessionToken {
    authorized_passphrases: Vec<PassphraseId>,
}

impl SessionToken {
    pub fn new() -> Self {
        SessionToken {
            authorized_passphrases: vec![],
        }
    }

    pub fn add_authorization(&mut self, passphrase_id: PassphraseId) {
        self.authorized_passphrases.push(passphrase_id)
    }

    pub fn get_passphrase_ids(&self) -> &[PassphraseId] {
        &self.authorized_passphrases
    }

    pub fn as_string(&self, secret: &str) -> String {
        let key = derive_key_from_secret(secret);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time is after Unix epoch")
            .as_millis();
        let mut msg: Vec<u8> = (timestamp as u64).to_le_bytes().into();
        msg.extend(
            self.authorized_passphrases
                .iter()
                .flat_map(|id| id.to_le_bytes()),
        );
        let tag = ring::hmac::sign(&key, &msg);
        let mut result: Vec<u8> = tag.as_ref().into();
        result.append(&mut msg);
        base64::engine::general_purpose::STANDARD.encode(&result)
    }

    pub fn from_string(
        data: &str,
        secret: &str,
        max_age: std::time::Duration,
    ) -> Result<Self, SessionError> {
        let tag_len = HMAC_ALGORITHM.digest_algorithm().output_len();
        let timestamp_len = std::mem::size_of::<u64>();
        let passphrase_id_len = std::mem::size_of::<PassphraseId>();
        let key = derive_key_from_secret(secret);

        let binary_data = base64::engine::general_purpose::STANDARD.decode(data)?;
        if binary_data.len() < tag_len + timestamp_len {
            return Err(SessionError::InvalidTokenStructure);
        }
        if (binary_data.len() - tag_len - timestamp_len) % passphrase_id_len != 0 {
            return Err(SessionError::InvalidTokenStructure);
        }
        let msg = &binary_data[tag_len..];
        let tag = &binary_data[0..tag_len];
        if ring::hmac::verify(&key, msg, tag).is_err() {
            return Err(SessionError::SignatureVerificationFailed);
        }
        let timestamp = std::time::UNIX_EPOCH
            + std::time::Duration::from_millis(u64::from_le_bytes(
                msg[0..timestamp_len].try_into().expect(
                    "timestamp_len should be the correct number of bytes and \
                           we should have checked before that the message is long enough.",
                ),
            ));
        let age = std::time::SystemTime::now()
            .duration_since(timestamp)
            .map_err(|_| SessionError::ExpiredToken)?;
        if age > max_age {
            return Err(SessionError::ExpiredToken);
        }

        let authorized_passphrases = msg[timestamp_len..]
            .chunks(passphrase_id_len)
            .map(|id_bytes| {
                PassphraseId::from_le_bytes(id_bytes.try_into().expect(
                    "passphrase_id_len should be the correct number of bytes and \
                           we should have checked before that the message is a multiple of it.",
                ))
            })
            .collect();

        Ok(Self {
            authorized_passphrases,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SessionError {
    InvalidTokenEncoding,
    InvalidTokenStructure,
    SignatureVerificationFailed,
    ExpiredToken,
}

impl From<base64::DecodeError> for SessionError {
    fn from(_value: DecodeError) -> Self {
        SessionError::InvalidTokenEncoding
    }
}

fn derive_key_from_secret(secret: &str) -> Key {
    assert_eq!(KEY_LENGTH, HMAC_ALGORITHM.digest_algorithm().block_len());
    let mut key_data = [0u8; KEY_LENGTH];
    ring::pbkdf2::derive(
        ring::pbkdf2::PBKDF2_HMAC_SHA256,
        10000.try_into().expect("10000 is not zero"),
        &[],
        secret.as_bytes(),
        &mut key_data,
    );
    ring::hmac::Key::new(HMAC_ALGORITHM, &key_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_AGE: std::time::Duration = std::time::Duration::from_secs(1 * 86400 * 365);

    #[test]
    fn empty_session() {
        const SECRET: &str = "abcdef";
        let session_token_str = SessionToken::new().as_string(SECRET);
        let decoded_token = SessionToken::from_string(&session_token_str, SECRET, MAX_AGE)
            .expect("Session token should be valid");
        let expected: &[PassphraseId] = &[];
        assert_eq!(decoded_token.get_passphrase_ids(), expected);
    }

    #[test]
    fn simple_session() {
        const SECRET: &str = "abcdef";
        let mut token = SessionToken::new();
        token.add_authorization(314);
        token.add_authorization(1024);
        let session_token_str = token.as_string(SECRET);
        let decoded_token = SessionToken::from_string(&session_token_str, SECRET, MAX_AGE)
            .expect("Session token should be valid");
        assert_eq!(decoded_token.get_passphrase_ids(), &[314, 1024]);
    }

    #[test]
    fn changed_secret() {
        const SECRET1: &str = "abcdef";
        const SECRET2: &str = "abcdff";
        let mut token = SessionToken::new();
        token.add_authorization(314);
        let session_token_str = token.as_string(SECRET1);
        let result = SessionToken::from_string(&session_token_str, SECRET2, MAX_AGE);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            SessionError::SignatureVerificationFailed
        );
    }

    #[test]
    fn tempered_token_added_auth() {
        const SECRET: &str = "abcdef";
        let mut token = SessionToken::new();
        token.add_authorization(314);
        let session_token_str = token.as_string(SECRET);

        // tempering
        let mut data = base64::engine::general_purpose::STANDARD
            .decode(session_token_str)
            .expect("data should be base64-decodable");
        data.extend(&315i32.to_le_bytes());

        let tempered_session_token_str = base64::engine::general_purpose::STANDARD.encode(data);
        let result = SessionToken::from_string(&tempered_session_token_str, SECRET, MAX_AGE);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            SessionError::SignatureVerificationFailed
        );
    }

    #[test]
    fn tempered_token_changed_auth() {
        const SECRET: &str = "abcdef";
        let mut token = SessionToken::new();
        token.add_authorization(314);
        let session_token_str = token.as_string(SECRET);

        // tempering
        let mut data = base64::engine::general_purpose::STANDARD
            .decode(session_token_str)
            .expect("data should be base64-decodable");
        // replacing the 314 with a 315
        data.truncate(data.len() - 4);
        data.extend(&315i32.to_le_bytes());

        let tempered_session_token_str = base64::engine::general_purpose::STANDARD.encode(data);
        let result = SessionToken::from_string(&tempered_session_token_str, SECRET, MAX_AGE);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            SessionError::SignatureVerificationFailed
        );
    }

    #[test]
    fn expired_token() {
        const SECRET: &str = "abcdef";
        let mut token = SessionToken::new();
        token.add_authorization(314);
        let session_token_str = token.as_string(SECRET);
        std::thread::sleep(std::time::Duration::from_millis(150));
        let result = SessionToken::from_string(
            &session_token_str,
            SECRET,
            std::time::Duration::from_millis(100),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::ExpiredToken);
    }
}
