# pgmorph

Zero-downtime schema migrations for PostgreSQL.

`pgmorph` is a CLI tool and library designed to perform PostgreSQL schema changes safely on production databases. It focuses on minimizing lock contention by using modern PostgreSQL features and best practices for high-availability environments.

## Key Features

- **Safe Constraints:** Adds `CHECK` and `FOREIGN KEY` constraints using `NOT VALID` and subsequent `VALIDATE CONSTRAINT` to avoid long-held access exclusive locks.
- **NotNull Migration:** Safely sets `NOT NULL` on existing columns by first adding a check constraint and then updating the column metadata.
- **Lock Management:** Configurable lock timeouts and retry logic to ensure migrations eventually succeed without impacting database performance.
- **Dry Run Mode:** Preview the SQL that would be executed without applying any changes.

## CLI Usage

### Global Options

- `--database-url`: PostgreSQL connection string (can also be set via `DATABASE_URL` environment variable).
- `--dry-run`: Preview SQL without executing.
- `--lock-timeout`: Maximum time to wait for a lock (default: `5s`).
- `--max-attempts`: Maximum number of retries for lock-sensitive operations (default: `5`).

### Commands

#### Introspect
Inspect a table's schema.
```bash
pgmorph introspect --table <table_name> [--schema <schema_name>]
```

#### Add Column
Add a new column to a table.
```bash
pgmorph add-column --table <table_name> --column <column_name> --type <data_type> [--default <value>] [--not-null]
```

#### Add Check
Add a check constraint safely.
```bash
pgmorph add-check --table <table_name> --constraint-name <name> --rule <sql_expression>
```

#### Add Foreign Key
Add a foreign key constraint safely.
```bash
pgmorph add-foreign-key --table <table_name> --column <column_name> --foreign-table-name <target_table> --foreign-column-name <target_column> --on-delete <action> [--constraint-name <name>]
```

#### Set Not Null
Safely set a column as `NOT NULL`.
```bash
pgmorph set-not-null-args --table <table_name> --column <column_name>
```

## Library API

`pgmorph` can also be used as a Rust library.

```rust
use pgmorph::db::{connect, direct::{add_column, AddColumnArgs}};
use pgmorph::config::MigrationConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = connect("host=localhost user=postgres").await?;
    let config = MigrationConfig::default();
    
    let args = AddColumnArgs {
        schema: "public".to_string(),
        table: "users".to_string(),
        column: "email".to_string(),
        data_type: "text".to_string(),
        not_null: false,
        default: None,
    };

    add_column(&client, &config, &args).await?;
    
    Ok(())
}
```

## Installation

```bash
cargo install --path .
```

## License

MIT
