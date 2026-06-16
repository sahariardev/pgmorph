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
        Some(Command::AddCheck {
            table,
            schema,
            constraint_name,
            rule,
        }) => {
            let args = db::direct::AddCheckArgs {
                schema,
                table,
                constraint_name,
                rule,
            };

            db::direct::handle_add_check(&client, &migration_config, &args).await?;
        }
        Some(Command::AddForeignKey {
            table,
            schema,
            constraint_name,
            column,
            foreign_table_name,
            foreign_column_name,
            on_delete,
        }) => {
            let args = db::direct::AddForeignKeyArgs {
                schema,
                table,
                constraint_name,
                column,
                foreign_table_name,
                foreign_column_name,
            };

            db::direct::handle_add_foreign_keys(&client, &migration_config, &args).await?;
        }
        Some(Command::SetNotNull {
            table,
            schema,
            column,
        }) => {
            let args = db::direct::AddNonNullArgs {
                schema,
                table,
                column,
            };

            db::direct::handle_add_non_null_keys(&client, &migration_config, &args).await?;
        }
    }

    Ok(())
}
