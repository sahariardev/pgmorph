mod cli;
mod db;
mod config;

use crate::cli::Command;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse();
    let client = db::connect(&cli.database_url).await?;

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
    }

    Ok(())
}
