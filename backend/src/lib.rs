mod data_store;

mod auth_session;
pub mod file_io;
pub mod web;

pub struct CliAuthToken {
    _private: (),
}

impl CliAuthToken {
    #[allow(clippy::new_without_default)] // We always want to explicitly create these objects
    pub fn new() -> Self {
        Self { _private: () }
    }
}
