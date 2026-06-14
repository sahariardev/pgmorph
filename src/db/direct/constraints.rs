use crate::config::MigrationConfig;
use crate::db::retry::{run_ddl_with_retry, RetryError};
use std::fmt::{Display, Formatter};
use tokio_postgres::Client;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConstraintState {
    Missing,
    Valid,
    Invalid,
}

#[derive(Debug)]
pub enum AddConstraintError {
    InvalidIdentifier { name: String, reason: String },
    Retry(RetryError),
    Database(tokio_postgres::Error),
    InvalidState(String),
}

impl Display for AddConstraintError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AddConstraintError::InvalidIdentifier { name, reason } => {
                write!(f, "invalid identifier '{}': {}", name, reason)
            }
            AddConstraintError::Retry(e) => write!(f, "retry error: {}", e),
            AddConstraintError::Database(e) => write!(f, "database error: {}", e),
            AddConstraintError::InvalidState(s) => write!(f, "invalid state: {}", s),
        }
    }
}

impl std::error::Error for AddConstraintError {}

impl From<RetryError> for AddConstraintError {
    fn from(e: RetryError) -> Self {
        AddConstraintError::Retry(e)
    }
}

impl From<tokio_postgres::Error> for AddConstraintError {
    fn from(value: tokio_postgres::Error) -> Self {
        AddConstraintError::Database(value)
    }
}

#[derive(Debug, Clone)]
pub struct AddCheckArgs {
    pub schema: String,
    pub table: String,
    pub constraint_name: String,
    pub rule: String,
}
pub async fn add_check(
    client: &Client,
    config: &MigrationConfig,
    args: &AddCheckArgs,
) -> Result<(), AddConstraintError> {
    let constraint_name = resolve_add_check_constraint_name(args)?;
    let add_constraint_query = resolve_add_check_phase_one_query(args, &constraint_name)?;
    let constraint_validate_query =
        resolve_constraint_validate_query(&args.schema, &args.table, &constraint_name)?;

    let drop_chek_query = resolve_drop_check_query(&args, &constraint_name)?;

    if config.dry_run {
        println!("-- dry-run --");
        run_ddl_with_retry(client, &add_constraint_query, config).await?;
        run_ddl_with_retry(client, &constraint_validate_query, config).await?;
        return Ok(());
    }

    for attempt in 1..=config.max_attempts {
        match fetch_constraint_state(client, &constraint_name, &args.table, &args.schema).await? {
            ConstraintState::Valid => {
                return Ok(());
            }
            ConstraintState::Invalid => {
                run_ddl_with_retry(client, &constraint_validate_query, config).await?;
                continue;
            }
            ConstraintState::Missing => {
                //expected
            }
        }

        run_ddl_with_retry(client, &add_constraint_query, config).await?;
        _ = run_ddl_with_retry(client, &constraint_validate_query, config).await;

        match fetch_constraint_state(client, &constraint_name, &args.table, &args.schema).await? {
            ConstraintState::Valid => {
                return Ok(());
            }
            ConstraintState::Invalid => {
                _ = run_ddl_with_retry(client, &drop_chek_query, config).await;
            }
            ConstraintState::Missing => {
                return Err(AddConstraintError::InvalidState(
                    "Invalid State".to_string(),
                ));
            }
        }
    }

    Err(AddConstraintError::InvalidState(
        "Invalid state".to_string(),
    ))
}

async fn fetch_constraint_state(
    client: &Client,
    constraint_name: &str,
    table: &str,
    schema: &str,
) -> Result<ConstraintState, AddConstraintError> {
    let query = format!(
        "SELECT convalidated FROM pg_constraint WHERE conrelid = '\"{schema}\".\"{table}\"'::regclass AND conname = $1;",
    );

    let rows = client.query(&query, &[&constraint_name]).await?;

    match rows.len() {
        0 => Ok(ConstraintState::Missing),
        1 => {
            let is_valid: bool = rows[0].get(0);

            if is_valid {
                Ok(ConstraintState::Valid)
            } else {
                Ok(ConstraintState::Invalid)
            }
        }
        _ => Err(AddConstraintError::InvalidState(
            "multiple constraint found".to_string(),
        )),
    }
}

fn resolve_add_check_constraint_name(args: &AddCheckArgs) -> Result<String, AddConstraintError> {
    validate_identifier(&args.constraint_name)?;
    Ok(args.constraint_name.clone())
}

fn resolve_add_check_phase_one_query(
    args: &AddCheckArgs,
    constraint_name: &str,
) -> Result<String, AddConstraintError> {
    validate_identifier(&args.schema)?;
    validate_identifier(&args.table)?;
    validate_identifier(&args.rule)?;

    Ok(format!(
        "ALTER TABLE \"{}\".\"{}\" ADD CONSTRAINT {} CHECK {} NOT VALID",
        args.schema, args.table, constraint_name, args.rule
    ))
}

fn resolve_drop_check_query(
    args: &AddCheckArgs,
    constraint_name: &str,
) -> Result<String, AddConstraintError> {
    Ok(format!(
        "ALTER TABLE \"{}\".\"{}\" DROP CONSTRAINT {} CHECK {}",
        args.schema, args.table, constraint_name, args.rule
    ))
}

fn resolve_constraint_validate_query(
    schema: &str,
    table: &str,
    constraint_name: &str,
) -> Result<String, AddConstraintError> {
    Ok(format!(
        "ALTER TABLE \"{}\".\"{}\" VALIDATE CONSTRAINT {}",
        schema, table, constraint_name
    ))
}

fn validate_identifier(name: &str) -> Result<(), AddConstraintError> {
    if name.is_empty() {
        return Err(AddConstraintError::InvalidIdentifier {
            name: name.to_string(),
            reason: "identifier cannot be empty".to_string(),
        });
    }

    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(AddConstraintError::InvalidIdentifier {
            name: name.to_string(),
            reason: "invalid character in identifier".to_string(),
        });
    }

    Ok(())
}
