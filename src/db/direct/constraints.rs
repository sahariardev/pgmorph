use crate::config::MigrationConfig;
use crate::db::retry::{RetryError, run_ddl_with_retry};
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

#[derive(Debug, Clone)]
pub struct AddForeignKeyArgs {
    pub schema: String,
    pub table: String,
    pub constraint_name: Option<String>,
    pub column: String, //currently only one column allowed
    pub foreign_table_name: String,
    pub foreign_column_name: String,
}

#[derive(Debug, Clone)]
pub struct AddNonNullArgs {
    pub schema: String,
    pub table: String,
    pub column: String,
}

pub async fn handle_add_non_null_keys(
    client: &Client,
    config: &MigrationConfig,
    args: &AddNonNullArgs,
) -> Result<(), AddConstraintError> {
    let constraint_name = resolve_non_null_constraint_name(args)?;
    let add_constraint_query = resolve_add_check_phase_one_query(
        &args.schema,
        &args.table,
        &constraint_name,
        &format!("{} is not null", &args.column),
    )?;

    let add_non_null_constraint_query = resolve_non_null_constraint_query(&args.schema, &args.table, &args.column);

    handle_add_constraint(
        client,
        config,
        &constraint_name,
        &add_constraint_query,
        &args.schema,
        &args.table,
        Some(&add_non_null_constraint_query),
    )
    .await
}

pub async fn handle_add_foreign_keys(
    client: &Client,
    config: &MigrationConfig,
    args: &AddForeignKeyArgs,
) -> Result<(), AddConstraintError> {
    let constraint_name = resolve_add_foreign_key_constraint_name(args)?;
    let add_constraint_query = resolve_add_foreign_key_query(args, &constraint_name)?;
    handle_add_constraint(
        client,
        config,
        &constraint_name,
        &add_constraint_query,
        &args.schema,
        &args.table,
        None,
    )
    .await
}

pub async fn handle_add_check(
    client: &Client,
    config: &MigrationConfig,
    args: &AddCheckArgs,
) -> Result<(), AddConstraintError> {
    let constraint_name = resolve_add_check_constraint_name(args)?;
    let add_constraint_query =
        resolve_add_check_phase_one_query(&args.schema, &args.table, &constraint_name, &args.rule)?;
    handle_add_constraint(
        client,
        config,
        &constraint_name,
        &add_constraint_query,
        &args.schema,
        &args.table,
        None,
    )
    .await
}

async fn handle_add_constraint(
    client: &Client,
    config: &MigrationConfig,
    constraint_name: &str,
    add_constraint_query: &str,
    schema: &str,
    table: &str,
    post_constraint_migration_sql: Option<&str>,
) -> Result<(), AddConstraintError> {
    let constraint_validate_query =
        resolve_constraint_validate_query(schema, table, constraint_name)?;

    let drop_chek_query = resolve_drop_check_query(schema, table, constraint_name)?;

    if config.dry_run {
        println!("-- dry-run --");
        run_ddl_with_retry(client, &add_constraint_query, config).await?;
        run_ddl_with_retry(client, &constraint_validate_query, config).await?;
        if let Some(query) = post_constraint_migration_sql {
            run_ddl_with_retry(client, query, config).await?;
        }
        return Ok(());
    }

    for attempt in 1..=config.max_attempts {
        match fetch_constraint_state(client, &constraint_name, table, schema).await? {
            ConstraintState::Valid => {
                if let Some(query) = post_constraint_migration_sql {
                    run_ddl_with_retry(client, query, config).await?;
                }
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

        match fetch_constraint_state(client, &constraint_name, table, schema).await? {
            ConstraintState::Valid => {
                if let Some(query) = post_constraint_migration_sql {
                    run_ddl_with_retry(client, query, config).await?;
                }
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

fn resolve_add_foreign_key_constraint_name(
    args: &AddForeignKeyArgs,
) -> Result<String, AddConstraintError> {
    if let Some(constraint_name) = &args.constraint_name {
        validate_identifier(constraint_name)?;
        return Ok(constraint_name.clone());
    }
    Ok(format!(
        "{}_{}_{}_fkey",
        args.table, args.foreign_table_name, args.foreign_column_name
    ))
}

fn resolve_non_null_constraint_name(args: &AddNonNullArgs) -> Result<String, AddConstraintError> {
    Ok(format!("{}_{}_nonNull", args.table, args.column))
}

fn resolve_add_check_phase_one_query(
    schema: &str,
    table: &str,
    constraint_name: &str,
    rule: &str,
) -> Result<String, AddConstraintError> {
    validate_identifier(schema)?;
    validate_identifier(table)?;
    validate_identifier(rule)?;

    Ok(format!(
        "ALTER TABLE \"{}\".\"{}\" ADD CONSTRAINT {} CHECK {} NOT VALID;",
        schema, table, constraint_name, rule
    ))
}

fn resolve_add_foreign_key_query(
    args: &AddForeignKeyArgs,
    constraint_name: &str,
) -> Result<String, AddConstraintError> {
    validate_identifier(&args.schema)?;
    validate_identifier(&args.table)?;
    validate_identifier(&args.foreign_table_name)?;
    validate_identifier(&args.foreign_column_name)?;
    validate_identifier(&args.column)?;

    Ok(format!(
        "ALTER TABLE \"{}\".\"{}\" ADD CONSTRAINT {} FOREIGN KEY {} REFERENCE {}({}) NOT VALID;",
        args.schema,
        args.table,
        constraint_name,
        args.column,
        args.foreign_table_name,
        args.foreign_column_name,
    ))
}

fn resolve_drop_check_query(
    schema: &str,
    table: &str,
    constraint_name: &str,
) -> Result<String, AddConstraintError> {
    Ok(format!(
        "ALTER TABLE \"{}\".\"{}\" DROP CONSTRAINT {}",
        schema, table, constraint_name
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

fn resolve_non_null_constraint_query(
    schema: &str,
    table: &str,
    column: &str,
) -> String {
    format!(
        "ALTER TABLE \"{}\".\"{}\" ALTER COLUMN {} SET NOT NULL",
        schema, table, column
    )
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
