
use tokio_postgres::Client;

pub async fn connect(db_url: &str) -> Result<Client, tokio_postgres::Error> {
    todo!("create connection and return the connection")
}