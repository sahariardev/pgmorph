mod cli;
mod config;
mod db;

use crate::cli::Command;
use crate::config::MigrationConfig;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse();
    let client = db::connect(&cli.database_url).await?;
    let migration_config = MigrationConfig::from_cli(&cli);

    match cli.command {
        None => {
            let row = client.query_one("SELECT version()", &[]).await?;
            let version: &str = row.get(0);

            print!("Connected to postgress");
            println!("version: {}", version);
        }
        Some(Command::Introspect { table, schema }) => {
            let info = db::introspect_table(&client, &schema, &table).await?;
            println!("{}", db::format_table_info(&info));
        }
        Some(Command::AddColumn {
            table,
            schema,
            column,
            data_type,
            default,
            not_null,
        }) => {
            let args = db::AddColumnArgs {
                table,
                schema,
                column,
                default,
                data_type,
                not_null,
            };
            db::add_column(&client, &migration_config, &args).await?;
        }
    }

    Ok(())
}
