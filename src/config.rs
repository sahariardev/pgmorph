use crate::cli::Cli;

#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub lock_timeout: String,
    pub max_attempts: u32,
    pub base_backoff_ms: u64,
    pub dry_run: bool,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            lock_timeout: "5s".to_string(),
            max_attempts: 5,
            base_backoff_ms: 500,
            dry_run: false,
        }
    }
}

impl MigrationConfig {
    pub fn from_cli(cli: &Cli) -> Self {
        Self {
            lock_timeout: cli.lock_timeout.clone(),
            max_attempts: cli.max_attempts,
            base_backoff_ms: 500,
            dry_run: cli.dry_run,
        }
    }
}