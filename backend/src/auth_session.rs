use crate::data_store::PassphraseId;

pub struct SessionToken {
    authorized_passphrases: Vec<PassphraseId>,
}

impl SessionToken {
    pub fn new() -> Self {
        SessionToken{authorized_passphrases: vec![]}
    }

    pub fn from_string(data: &str, secret: &str) -> Result<Self, SessionError> {
        todo!()
    }

    pub fn as_string(&self, secret: &str) -> String {
        todo!()
    }

    pub fn add_authorization(&mut self, passphrase_id: PassphraseId) {
        self.authorized_passphrases.push(passphrase_id)
    }

    pub fn get_passphrase_ids(&self) -> &[PassphraseId] {
        &self.authorized_passphrases
    }
}

pub enum SessionError {
    InvalidTokenFormat,
    SignatureVerificationFailed,
    ExpiredToken,
}
