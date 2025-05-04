mod data_store;

mod auth_session;
pub mod cli;
pub mod cli_error;
mod setup;
pub mod web;

fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
