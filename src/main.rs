use clap::Parser;
use tokio_postgres::NoTls;

const DEFAULT_DATABASE_URL: &str = "host=localhost port=5433 user=pgmorph password=pgmorph dbname=pgmorph";

#[derive(Parser, Debug)]
#[command(name = "pgmorph", about = "Zero-downtime schema migration for postgresql")]
struct Cli {
    #[arg(long, env = "DATABASE_URL", default_value = DEFAULT_DATABASE_URL)]
    database_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let (client, connection) = tokio_postgres::connect(&cli.database_url, NoTls).await?;
    //connect to db
    //select version

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let row = client.query_one("SELECT version()", &[]).await?;
    let version : &str = row.get(0);

    print!("Connected to postgress");
    println!("version: {}", version);

    Ok(())
}