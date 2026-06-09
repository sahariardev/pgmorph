use crate::db::retry::RetryError;

#[derive(Debug, Clone, PartialEq, Eq)]
enum IndexState {
    Missing,
    Valid,
    Invalid,
}

#[derive(Debug)]
pub enum AddIndexError {
    InvalidIdentifier { name: String, reason: String },
    InvalidColumns { message: String },
    Retry(RetryError),
    InvalidIndexPersisted { index_name: String, attempts: u32 },
    Database(tokio_postgres::Error),
}

impl std::fmt::Display for AddIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidIdentifier { name, reason } => {
                write!(f, "invalid identifier '{}': {}", name, reason)
            }
            Self::InvalidColumns { message } => {
                write!(f, "{message}")
            }
            Self::Retry(e) => write!(f, "{}", e),
            Self::InvalidIndexPersisted {
                index_name,
                attempts,
            } => {
                write!(f, "index {index_name} already exists at attempt {attempts}")
            }
            Self::Database(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for AddIndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Retry(e) => Some(e),
            Self::Database(e) => Some(e),
            _ => None,
        }
    }
}

impl From<RetryError> for AddIndexError {
    fn from(value: RetryError) -> Self {
        Self::Retry(value)
    }
}

impl From<tokio_postgres::Error> for AddIndexError {
    fn from(value: tokio_postgres::Error) -> Self {
        Self::Database(value)
    }
}

#[derive(Debug, Clone)]
pub struct AddIndexArgs {
    pub schema: String,
    pub table: String,
    pub index_name: Option<String>,
    pub columns: Vec<String>,
    pub unique: bool,
}

pub fn resolve_index_name(args: &AddIndexArgs) -> Result<String, AddIndexError> {
    if let Some(index_name) = &args.index_name {
        validate_identifier(index_name)?;
        return Ok(index_name.clone());
    }

    if args.columns.is_empty() {
        return Err(AddIndexError::InvalidColumns {
            message: "index must not be empty".to_string(),
        });
    }

    Ok(default_index_name(&args.table, &args.columns))
}

fn validate_identifier(name: &str) -> Result<(), AddIndexError> {
    if name.is_empty() {
        return Err(AddIndexError::InvalidIdentifier {
            name: name.to_string(),
            reason: "identifier cannot be empty".to_string(),
        });
    }

    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(AddIndexError::InvalidIdentifier {
            name: name.to_string(),
            reason: "identifier must start with an underscore".to_string(),
        });
    }

    Ok(())
}

fn default_index_name(table: &str, columns: &[String]) -> String {
    format!("{}_{}_idx", table, columns.join("_"))
}
