use tokio_postgres::NoTls;

const DB_URL: &str =
    "host=localhost port=5433 user=pgmorph password=pgmorph dbname=pgmorph";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (client, connection) =
        tokio_postgres::connect(DB_URL, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
    });

    client.execute(
        "
        CREATE TABLE IF NOT EXISTS orders (
            id BIGSERIAL PRIMARY KEY,
            customer_id BIGINT NOT NULL,
            amount DECIMAL(10,2) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        ",
        &[],
    ).await?;

    client.execute(
        "
        CREATE INDEX IF NOT EXISTS idx_orders_customer_id
        ON orders(customer_id);
        ",
        &[],
    ).await?;

    println!("Orders table and index created");

    Ok(())
}