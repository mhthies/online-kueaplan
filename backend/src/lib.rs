mod api;
mod data_store;

mod auth_session;
pub mod file_io;
pub mod web;

pub struct CliAuthToken{
    _private: (),
}

impl CliAuthToken{
    pub fn new() -> Self{
        Self { _private: () }
    }
}
