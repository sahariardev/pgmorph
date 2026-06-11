use std::fmt::format;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use pgmorph::db::connect;

#[tokio::test]
async fn add_index_builds_valid_index_under_concurrent_insers() {
    let postgres = Postgres::default().start().await.expect("Start postgres");

    let host = postgres.get_host().await.expect("host");
    let port = postgres.get_host_port_ipv4(5432).await.expect("mapped port");

    let database_url = format!(
        "host={host} port={port} user=postgres password=postgres dbname=postgres"
    );

    let client = connect(&database_url).await.expect("connect main client");
    let writer = connect(&database_url).await.expect("connect writer");
}
