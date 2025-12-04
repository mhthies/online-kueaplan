use crate::data_store::StoreError;
use crate::setup::SetupError;
use diesel::ConnectionError;

#[derive(Debug)]
pub enum CliError {
    /// The application setup (environment variables) are not complete or invalid
    SetupError(String),
    /// Could not connect to the database server
    CouldNotConnectToDatabase(String),
    /// Somehow, the database connection or our data_store abstraction failed during startup or cli
    /// data transactions
    UnexpectedStoreError(String),
    /// Binding the web server to the requested port failed
    BindError(std::io::Error),
    /// Starting the web server failed with an io error
    ServerError(std::io::Error),
    /// Somehow, migrating the database to the current schema version failed
    DatabaseMigrationError(String),
    /// Cannot start because one or more database schema migrations are pending
    DatabaseMigrationRequired {
        /// The names of the pending database schema migrations
        missing_migrations: Vec<String>,
    },
    /// Failure while handling some file for a cli data transaction
    FileError(String),
    /// Could not complete command because the provided data (e.g. an input file) is not valid
    DataError(String),
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
            CliError::BindError(_) => 3,
            CliError::ServerError(_) => 3,
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
            CliError::BindError(e) => {
                write!(f, "Could not bind web server socket to TCP port: {}", e)
            }
            CliError::ServerError(e) => write!(f, "Could not initialize web server: {}", e),
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
            StoreError::NotValid => Self::DataError("Item not valid".to_string()),
            StoreError::ConflictEntityExists => {
                Self::DataError("Conflicting entity exists".to_string())
            }
            StoreError::ConcurrentEditConflict => {
                Self::UnexpectedStoreError("Concurrent edit conflict".to_string())
            }
            StoreError::PermissionDenied {
                required_privilege,
                event_id: Some(event_id),
                ..
            } => Self::UnexpectedStoreError(format!(
                "Missing data_store privilege: {:?} for event {}",
                required_privilege, event_id
            )),
            StoreError::PermissionDenied {
                required_privilege,
                event_id: None,
                ..
            } => Self::UnexpectedStoreError(format!(
                "Missing global data_store privilege: {:?}",
                required_privilege
            )),
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

impl From<SetupError> for CliError {
    fn from(value: SetupError) -> Self {
        Self::SetupError(value.to_string())
    }
}

impl From<diesel::ConnectionError> for CliError {
    fn from(value: ConnectionError) -> Self {
        Self::CouldNotConnectToDatabase(value.to_string())
    }
}
