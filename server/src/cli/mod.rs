use crate::data_store::StoreError;

pub mod database_migration;
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

#[derive(Debug)]
pub enum CliError {
    SetupError(String),
    CouldNotConnectToDatabase(String),
    DatabaseMigrationRequired { missing_migrations: Vec<String> },
    DataError(String),
    FileError(String),
    DatabaseMigrationError(String),
    UnexpectedStoreError(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::SetupError { .. } => 1,
            CliError::CouldNotConnectToDatabase(_) => 4,
            CliError::DatabaseMigrationRequired { .. } => 5,
            CliError::DataError(_) => 1,
            CliError::FileError(_) => 1,
            CliError::DatabaseMigrationError(_) => 4,
            CliError::UnexpectedStoreError(_) => 2,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::SetupError(e) => {
                write!(f, "Setup invalid: {}", e)
            }
            CliError::CouldNotConnectToDatabase(e) => {
                write!(f, "Could not connect to database: {}", e)
            }
            CliError::DatabaseMigrationRequired { missing_migrations } => {
                write!(
                    f,
                    "Database migration required. Missing migrations: {}",
                    missing_migrations.join(", ")
                )
            }
            CliError::DataError(e) => {
                write!(f, "Provided data is invalid: {}", e)
            }
            CliError::FileError(e) => f.write_str(e),
            CliError::DatabaseMigrationError(e) => {
                write!(f, "Error while applying database migrations: {}", e)
            }
            CliError::UnexpectedStoreError(e) => {
                write!(f, "Unexpected error in data store: {}", e)
            }
        }
    }
}

impl From<StoreError> for CliError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::ConnectionError(e) => Self::CouldNotConnectToDatabase(e),
            StoreError::QueryError(e) => Self::UnexpectedStoreError(e.to_string()),
            StoreError::TransactionConflict => {
                Self::UnexpectedStoreError("Concurrent transaction conflict".to_string())
            }
            StoreError::NotExisting => Self::DataError("Item not existing".to_string()),
            StoreError::ConflictEntityExists => {
                Self::DataError("Conflicting entity exists".to_string())
            }
            StoreError::PermissionDenied { required_privilege } => Self::UnexpectedStoreError(
                format!("Missing data_store privilege: {:?}", required_privilege),
            ),
            StoreError::InvalidInputData(e) => Self::DataError(e),
            StoreError::InvalidDataInDatabase(e) => Self::UnexpectedStoreError(e),
        }
    }
}

impl From<serde_json::Error> for CliError {
    fn from(value: serde_json::Error) -> Self {
        Self::DataError(value.to_string())
    }
}
