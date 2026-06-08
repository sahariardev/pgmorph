use crate::db::retry::RetryError;
use log::error;
use tokio_postgres::Client;
use crate::config::MigrationConfig;

#[derive(Debug)]
pub enum AddColumnError {
    InvalidIdentifier { name: String, reason: String },
    Unsupported { message: String },
    Retry(RetryError),
}

impl std::fmt::Display for AddColumnError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidIdentifier { name, reason } => {
                write!(f, "invalid identifier '{}' in '{}'", name, reason)
            }
            Self::Unsupported { message } => {
                write!(f, "{}", message)
            }
            Self::Retry(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for AddColumnError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Retry (error)=> Some(error),
            _ => None,
        }
    }
}

impl From<RetryError> for AddColumnError {
    fn from(e: RetryError) -> Self {
        Self::Retry(e)
    }
}

#[derive(Debug, Clone)]
pub struct AddColumnArgs {
    pub schema: String,
    pub table: String,
    pub column: String,
    pub data_type: String,
    pub not_null: bool,
    pub default: Option<String>,
}

pub async fn add_column(client: &Client, config: &MigrationConfig) -> Result<(), AddColumnError> {
    todo!("Implement this")
}