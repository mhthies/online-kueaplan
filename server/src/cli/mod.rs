pub mod database_migration;
pub mod file_io;
pub mod manage_events;
pub mod manage_passphrases;
mod util;

pub struct CliAuthTokenKey {
    _private: (),
}

impl CliAuthTokenKey {
    #[allow(clippy::new_without_default)] // We always want to explicitly create these objects
    pub(in crate::cli) fn new() -> Self {
        Self { _private: () }
    }
}

/// Union-type for event id or event slug, to be used as a command-line argument for specifying an
/// event.
///
/// When the given command-line argument is an integer, it is assumed to be an event id, otherwise
/// it is assumed to be an event slug.
#[derive(Debug, Clone)]
pub enum EventIdOrSlug {
    Id(i32),
    Slug(String),
}

impl From<String> for EventIdOrSlug {
    fn from(value: String) -> Self {
        value
            .parse::<i32>()
            .map(EventIdOrSlug::Id)
            .unwrap_or(EventIdOrSlug::Slug(value))
    }
}
