pub mod file_io;

pub struct CliAuthTokenKey {
    _private: (),
}

impl CliAuthTokenKey {
    #[allow(clippy::new_without_default)] // We always want to explicitly create these objects
    pub fn new() -> Self {
        Self { _private: () }
    }
}
