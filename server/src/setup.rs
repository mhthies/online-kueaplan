use diesel::dsl::Set;
use std::env;
use std::env::VarError;
use std::fmt::{Display, Formatter};

/// Get the database URL from the environment variable.
pub fn get_database_url_from_env() -> Result<String, SetupError> {
    env::var("DATABASE_URL").map_err(|e| SetupError::from_env_error(e, "DATABASE_URL"))
}

/// Get the cryptographic application secret for signing secure tokens from the environment variable.
pub fn get_secret_from_env() -> Result<String, SetupError> {
    env::var("SECRET").map_err(|e| SetupError::from_env_error(e, "SECRET"))
}

/// Get the web server TCP listening port from the environment variable
pub fn get_listen_port_from_env() -> Result<u16, SetupError> {
    env::var("LISTEN_PORT")
        .map_err(|e| SetupError::from_env_error(e, "LISTEN_PORT"))
        .and_then(|v| {
            v.parse().map_err(|e| SetupError::EnvVariableInvalid {
                variable_name: "LISTEN_PORT",
                problem: "Not a valid uint16",
            })
        })
}

/// Get the web server TCP listening interface address from the environment variable
pub fn get_listen_address_from_env() -> Result<String, SetupError> {
    env::var("LISTEN_ADDRESS").map_err(|e| SetupError::from_env_error(e, "LISTEN_ADDRESS"))
}

#[derive(Debug)]
pub enum SetupError {
    EnvVariableMissing {
        variable_name: &'static str,
    },
    EnvVariableInvalid {
        variable_name: &'static str,
        problem: &'static str,
    },
}

impl SetupError {
    fn from_env_error(error: VarError, variable_name: &'static str) -> Self {
        match error {
            VarError::NotPresent => Self::EnvVariableMissing { variable_name },
            VarError::NotUnicode(_) => Self::EnvVariableInvalid {
                variable_name,
                problem: "no valid unicode",
            },
        }
    }
}

impl Display for SetupError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SetupError::EnvVariableMissing { variable_name } => {
                write!(f, "Environment variable {} must be defined", variable_name)
            }
            SetupError::EnvVariableInvalid {
                variable_name,
                problem,
            } => write!(
                f,
                "Value of environment variable {} is invalid: {}",
                variable_name, problem
            ),
        }
    }
}
