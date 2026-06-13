use crate::config::MigrationConfig;
use crate::db::retry::{run_ddl_with_retry, RetryError};
use std::fmt::Pointer;
use tokio_postgres::Client;
use crate::db;

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

pub async fn add_index(
    client: &Client,
    config: &MigrationConfig,
    args: &AddIndexArgs,
) -> Result<(), AddIndexError> {
    let index_name = resolve_index_name(args)?;
    let index_query = build_create_index_sql(args, &index_name)?;
    let drop_query = build_drop_index_sql(&args.schema, &index_name)?;
    let validity_sql = build_validity_check_sql(&args.schema, &index_name);

    if config.dry_run {
        println!("-- dry-run --");
        run_ddl_with_retry(client, &index_query, config).await?;
        return Ok(());
    }

    for attempt in 1..=config.max_attempts {
        match fetch_index_state(client, &args.schema, &index_name).await? {
            IndexState::Valid => {
                println!("{} is already exists and valid", index_name);
            }
            IndexState::Invalid => {
                eprintln!(
                    "attempt {attempt}/{}: found invalid index '{index_name}'\
                . Dropping before recreating them",
                    config.max_attempts
                );

                run_ddl_with_retry(client, &drop_query, config).await?;
            }
            IndexState::Missing => {}
        }

        let create_result = run_ddl_with_retry(client, &index_query, config).await;

        match fetch_index_state(client, &args.schema, &index_name).await? {
            IndexState::Valid => {
                println!("Index created successfully");
                return Ok(());
            }
            IndexState::Invalid => {
                eprintln!(
                    "attempt {attempt}/{}: found invalid index '{index_name}'\"",
                    config.max_attempts
                );

                run_ddl_with_retry(client, &drop_query, config).await?;

                if attempt < config.max_attempts {
                    tokio::time::sleep(db::backoff_duration(attempt, config.base_backoff_ms)).await;
                }
            }
            IndexState::Missing => {
                return Err(
                    match create_result {
                        Err(error) => error.into(),
                        Ok(()) => AddIndexError::InvalidColumns {
                            message: format!(
                                "CREATE INDEX CONCURRENTLY finisehd but '{index_name}' not found"
                            )
                        }
                    }
                );
            }
        }
    }

    Err(AddIndexError::InvalidIndexPersisted {
        index_name,
        attempts: config.max_attempts,
    })
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

pub fn build_create_index_sql(
    args: &AddIndexArgs,
    index_name: &str,
) -> Result<String, AddIndexError> {
    validate_identifier(index_name)?;
    validate_identifier(&args.table)?;
    validate_identifier(&args.schema)?;

    if args.columns.is_empty() {
        return Err(AddIndexError::InvalidColumns {
            message: "At least one column is required".to_string(),
        });
    }

    for columns in &args.columns {
        validate_identifier(columns)?;
    }

    let unique = if args.unique { "UNIQUE " } else { "" };
    let qualified_table = format!("\"{}\".\"{}\"", args.schema, args.table);
    let column_list = args
        .columns
        .iter()
        .map(|column| format!("\"{column}\""))
        .collect::<Vec<_>>()
        .join(", ");

    Ok(format!(
        "CREATE {unique}INDEX CONCURRENTLY IF NOT EXISTS \"{index_name}\" ON {qualified_table} ({column_list})"
    ))
}

async fn fetch_index_state(
    client: &Client,
    schema: &str,
    index_name: &str,
) -> Result<IndexState, AddIndexError> {
    let rows = client
        .query(
            "SELECT i.indisvalid FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        JOIN pg_index i ON i.indexrelid = c.oid
        WHERE n.nspname = $1
        AND c.relname = $2
        AND c.relkind = 'i'",
            &[&schema, &index_name],
        )
        .await?;

    match rows.len() {
        0 => Ok(IndexState::Missing),
        1 => {
            let is_valid: bool = rows[0].get(0);
            if is_valid {
                Ok(IndexState::Valid)
            } else {
                Ok(IndexState::Invalid)
            }
        }
        _ => Err(AddIndexError::InvalidColumns {
            message: format!("multiple indexes name '{index_name}' in schema '{schema}'"),
        }),
    }
}
pub fn build_drop_index_sql(schema: &str, index_name: &str) -> Result<String, AddIndexError> {
    validate_identifier(schema)?;
    validate_identifier(&index_name)?;

    Ok(format!(
        "DROP INDEX CONCURRENTLY IF EXISTS \"{schema}\".\"{index_name}\""
    ))  
}

fn build_validity_check_sql(schema: &str, index_name: &str) -> String {
    format!(
        "SELECT indisvalid FROM pg_index WHERE indexrelid = '\"{schema}\".\"{index_name}\"'::regclass"
    )
}
