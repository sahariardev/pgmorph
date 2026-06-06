mod cli;
mod db;

use clap::Parser;
use crate::cli::Command;

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
        },
        Some(Command::Introspect {table}) => {
            println!("Introspect is not implemented yet!! passed table name is {}", table);
        }
    }

    Ok(())
}
