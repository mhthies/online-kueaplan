//! This module uses the embedded Diesel migration data to provide functions for checking the
//! database migration status and migrating the database schema to the current state.
//!
//! The functions provided functions are meant to be used directly from the command line interface
//! implementation.
use crate::data_store::get_database_url_from_env;
use diesel::migration::Migration;
use diesel::Connection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::fmt::{Debug, Display, Formatter};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/postgresql");

/// Migrate the database schema to the latest known migration for the current application version.
///
/// The database connection URL is taken from the environment variable, using
/// [get_database_url_from_env]. Information about the migration process is printed to stdout.
pub fn run_migrations() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut connection = diesel::pg::PgConnection::establish(&get_database_url_from_env()?)?;
    let mut connection =
        diesel_migrations::HarnessWithOutput::new(&mut connection, std::io::stdout());
    connection.run_pending_migrations(MIGRATIONS)?;

    Ok(())
}

#[derive(Debug)]
struct MigrationsStateOutdatedError {
    missing_migrations: Vec<String>,
}

impl Display for MigrationsStateOutdatedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Database needs to be migrated. Pending migrations: {}",
            self.missing_migrations.join(", ")
        ))
    }
}

impl std::error::Error for MigrationsStateOutdatedError {}

/// Check if the database schema has been migrated to the latest known migration for the current
/// application version. If not, return an error, describing the missing migrations.
///
/// The database connection URL is taken from the environment variable, using
/// [get_database_url_from_env].
pub fn check_migration_state() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut connection = diesel::pg::PgConnection::establish(&get_database_url_from_env()?)?;
    let mut connection =
        diesel_migrations::HarnessWithOutput::new(&mut connection, std::io::stdout());
    let pending_migrations = connection.pending_migrations(MIGRATIONS)?;
    if !pending_migrations.is_empty() {
        return Err(Box::new(MigrationsStateOutdatedError {
            missing_migrations: pending_migrations
                .iter()
                .map(|m| m.name().to_string())
                .collect(),
        }));
    }
    Ok(())
}
