use pgmorph::config::MigrationConfig;
use pgmorph::db::{add_column, connect, introspect_table, AddColumnArgs};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn test_add_column_with_default_and_not_null() {
    let postgres = Postgres::default().start().await.expect("Start postgres");

    let host = postgres.get_host().await.expect("host");
    let port = postgres
        .get_host_port_ipv4(5432)
        .await
        .expect("mapped port");

    let database_url =
        format!("host={host} port={port} user=postgres password=postgres dbname=postgres");

    let client = connect(&database_url).await.expect("connect main client");

    client
        .batch_execute(
            "CREATE TABLE users (
                id serial PRIMARY KEY,
                username text NOT NULL
            );",
        )
        .await
        .expect("create users table");

    let config = MigrationConfig {
        lock_timeout: "5s".to_string(),
        max_attempts: 5,
        base_backoff_ms: 200,
        dry_run: false,
    };

    let args = AddColumnArgs {
        schema: "public".to_string(),
        table: "users".to_string(),
        column: "email".to_string(),
        data_type: "text".to_string(),
        not_null: true,
        default: Some("'unknown@example.com'".to_string()),
    };

    add_column(&client, &config, &args)
        .await
        .expect("add_column failed");

    let table_info = introspect_table(&client, "public", "users")
        .await
        .expect("introspect table failed");

    let column = table_info
        .columns
        .iter()
        .find(|c| c.name == "email")
        .expect("column 'email' should exist");

    assert_eq!(column.data_type, "text");
    assert_eq!(column.nullable, false);
    assert!(column.default_value.as_ref().unwrap().contains("unknown@example.com"));
}
