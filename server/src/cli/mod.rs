pub mod database_migration;
pub mod file_io;
pub mod manage_events;

pub struct CliAuthTokenKey {
    _private: (),
}

impl CliAuthTokenKey {
    #[allow(clippy::new_without_default)] // We always want to explicitly create these objects
    pub(in crate::cli) fn new() -> Self {
        Self { _private: () }
    }
}
