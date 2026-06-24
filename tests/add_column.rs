mod common;

use common::*;
use pgmorph::config::MigrationConfig;
use pgmorph::db::{add_column, introspect_table, AddColumnArgs};
use testcontainers::runners::AsyncRunner;

fn cfg() -> MigrationConfig {
    MigrationConfig {
        lock_timeout: "5s".to_string(),
        max_attempts: 5,
        base_backoff_ms: 200,
        dry_run: false,
    }
}

fn args(table: &str, column: &str, data_type: &str) -> AddColumnArgs {
    AddColumnArgs {
        schema: "public".to_string(),
        table: table.to_string(),
        column: column.to_string(),
        data_type: data_type.to_string(),
        not_null: false,
        default: None,
    }
}

#[tokio::test]
async fn test_add_column() {
    let db = TestDB::start().await;

    db.client
        .batch_execute(
            "CREATE TABLE users (
                id serial PRIMARY KEY,
                username text NOT NULL
            );",
        )
        .await
        .expect("create users table");

    let config = cfg();
    let args = args("users", "email", "text");

    add_column(&db.client, &config, &args)
        .await
        .expect("add_column failed");

    let table_info = introspect_table(&db.client, "public", "users")
        .await
        .expect("introspect table failed");

    let column = table_info
        .columns
        .iter()
        .find(|c| c.name == "email")
        .expect("column 'email' should exist");

    assert_eq!(column.data_type, "text");
    assert_eq!(column.nullable, true);
}
