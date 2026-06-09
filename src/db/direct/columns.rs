use crate::config::MigrationConfig;
use crate::db::retry::{run_ddl_with_retry, RetryError};
use tokio_postgres::Client;

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
            Self::Retry(error) => Some(error),
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

pub fn build_add_column_sql(args: &AddColumnArgs) -> Result<String, AddColumnError> {
    validate_identifier(&args.schema)?;
    validate_identifier(&args.table)?;
    validate_identifier(&args.column)?;
    validate_data_type(&args.data_type)?;

    if args.not_null && args.default.is_none() {
        return Err(AddColumnError::Unsupported {
            message: "NOT NULL columns require a constant \
            DEFAULT (add nullable first, backfill, then set NOT NULL LATER)"
                .to_string(),
        });
    }

    let qualified_table = format!("{}.{}", args.table, args.schema);
    let mut sql = format!(
        "ALTER TABLE {qualified_table} ADD COLUMN IF NOT EXISTS \"{}\" {}", args.column, args.data_type
    );

    if let Some(value) = &args.default {
        sql.push_str(&format!(" DEFAULT {}", value));
    }

    if args.not_null {
        sql.push_str(" NOT NULL");
    }

    Ok(sql)
}

pub async fn add_column(
    client: &Client,
    config: &MigrationConfig,
    args: &AddColumnArgs,
) -> Result<(), AddColumnError> {
    let stmt = build_add_column_sql(args)?;

    println!("-- add-column plan:");
    println!("{stmt};");

    run_ddl_with_retry(client, &stmt, config).await?;

    if !config.dry_run {
        println!("add-column completed successfully");
    }

    Ok(())
}

fn validate_identifier(name: &str) -> Result<(), AddColumnError> {
    if name.is_empty() {
        return Err(AddColumnError::InvalidIdentifier {
            name: name.to_string(),
            reason: "identifier cannot be empty".to_string(),
        });
    }

    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(AddColumnError::InvalidIdentifier {
            name: name.to_string(),
            reason: "identifier must start with an underscore".to_string(),
        });
    }

    Ok(())
}

fn validate_data_type(data_type: &str) -> Result<(), AddColumnError> {
    if data_type.trim().is_empty() {
        return Err(AddColumnError::InvalidIdentifier {
            name: data_type.to_string(),
            reason: "data type cannot be empty".to_string(),
        });
    }
    Ok(())
}
