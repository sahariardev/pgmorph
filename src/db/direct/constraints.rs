// add-check
// check if check already exist or not
// if exist then return success
// if not then what actually it is returning
// if could exist partially
// if partially complete the phase two which
// if not then run phas1 and phase2
// if phase 1 failes continue
// if phase 2 fails cleanup

//add-foreignkey
//add-non-null

use crate::config::MigrationConfig;
use crate::db::retry::{RetryError, run_ddl_with_retry};
use std::fmt::{Display, Formatter};
use tokio_postgres::Client;

#[derive(Debug)]
pub enum AddConstraintError {
    InvalidIdentifier { name: String, reason: String },
    Retry(RetryError),
}

impl Display for AddConstraintError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AddConstraintError::InvalidIdentifier { name, reason } => {
                write!(f, "invalid identifier '{}': {}", name, reason)
            }
            AddConstraintError::Retry(e) => write!(f, "retry error: {}", e),
        }
    }
}

impl std::error::Error for AddConstraintError {}

impl From<RetryError> for AddConstraintError {
    fn from(e: RetryError) -> Self {
        AddConstraintError::Retry(e)
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

    if config.dry_run {
        println!("-- dry-run --");
        run_ddl_with_retry(client, &add_constraint_query, config).await?;
        run_ddl_with_retry(client, &constraint_validate_query, config).await?;
        return Ok(());
    }

    //check if this constraint already exist or not
    // if dry run only show queries

    todo!("Implement this")
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
