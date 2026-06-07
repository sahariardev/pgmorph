use crate::config::MigrationConfig;
use std::fmt::Formatter;
use std::time::Duration;
use tokio_postgres::Client;

pub const LOCK_TIMEOUT_SQLSTATE: &str = "55P03";

#[derive(Debug)]
pub enum RetryError {
    LockTimeoutExhausted { attempts: u32 },
    Database(tokio_postgres::Error),
}

impl std::fmt::Display for RetryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(error) => write!(f, "Database error: {}", error),
            Self::LockTimeoutExhausted { attempts } => {
                write!(f, "Lock timeout exhausted attempts: {}", attempts)
            }
        }
    }
}

impl std::error::Error for RetryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

pub fn is_lock_timeout(error: &tokio_postgres::Error) -> bool {
    error
        .as_db_error()
        .is_some_and(|db_error| db_error.code().code() == LOCK_TIMEOUT_SQLSTATE)
}

fn backoff_duration(attempt: u32, base_ms: u64) -> Duration {
    let exponent = attempt.saturating_sub(1).min(10);
    let base = base_ms.saturating_mul(1u64 << exponent);
    let jitter = (attempt as u64 * 137) % (base / 4 + 1);
    Duration::from_millis(base.saturating_add(jitter))
}

pub async fn run_ddl_with_retry(
    client: &Client,
    stmt: &str,
    config: &MigrationConfig,
) -> Result<(), RetryError> {
    if config.dry_run {
        println!("--dry-run: would execute the following statements");
        println!("SET lock_timeout = '{}';", config.lock_timeout);
        println!("{stmt};");
        return Ok(());
    }
    //if dry run, print stm and return
    // try to execute the stmt with a timeout and retry
    // only do retry for unable to get lock

    for attempt in 1..=config.max_attempts {
        if let Err(error) = client
            .execute(
                &format!("SET lock_timeout = '{}'", config.lock_timeout),
                &[],
            )
            .await
        {
            return Err(RetryError::Database(error));
        }

        match client.execute(stmt, &[]).await {
            Ok(_) => return Ok(()),
            Err(error) if is_lock_timeout(&error) => {
                eprintln!(
                    "attempt {attempt}/{}: lock timeout (SQLSTATE {LOCK_TIMEOUT_SQLSTATE}), retrying...",
                    config.max_attempts
                );

                if attempt < config.max_attempts {
                    tokio::time::sleep(backoff_duration(attempt, config.max_attempts as u64)).await;
                }
            }
            Err(error) => return Err(RetryError::Database(error)),
        }
    }

    Err(RetryError::LockTimeoutExhausted {
        attempts: config.max_attempts,
    })
}
