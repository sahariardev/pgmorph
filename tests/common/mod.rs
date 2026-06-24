use pgmorph::db::connect;
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::postgres::Postgres;
use tokio_postgres::Client;

pub struct TestDB {
    pub client: Client,
    _container: ContainerAsync<Postgres>,
}

impl TestDB {
    pub async fn start() -> TestDB {
        let postgres = Postgres::default().start().await.expect("Start postgres");

        let host = postgres.get_host().await.expect("host");
        let port = postgres
            .get_host_port_ipv4(5432)
            .await
            .expect("mapped port");

        let database_url =
            format!("host={host} port={port} user=postgres password=postgres dbname=postgres");

        let client = connect(&database_url).await.expect("connect");

        Self {
            client,
            _container: postgres,
        }
    }
    pub async fn second_client(&self) -> Client {
        let host = self._container.get_host().await.expect("host");
        let port = self
            ._container
            .get_host_port_ipv4(5432)
            .await
            .expect("mapped port");
        let database_url =
            format!("host={host} port={port} user=postgres password=postgres dbname=postgres",);

        connect(&database_url).await.expect("connect")
    }
}

pub async fn create_test_table(client: &Client, suffix: &str) -> String {
    let table_name = format!("t_{suffix}");
    client
        .batch_execute(&format!(
            "CREATE TABLE {table_name} (
                id bigserial PRIMARY KEY,
                amount numeric NOT NULL,
                status text,
                created_at timestamptz NOT NULL DEFAULT now()
            )"
        ))
        .await
        .unwrap_or_else(|e| panic!("Create table failed: {}", e));

    table_name
}

pub async fn create_fk_table(client: &Client, suffix: &str) -> (String, String) {
    let parent = format!("p_{suffix}");
    let child = format!("c_{suffix}");
    client
        .batch_execute(&format!(
            "CREATE TABLE {parent} (id bigserial PRIMARY KEY, name text NOT NULL);
             CREATE TABLE {child} (id bigserial PRIMARY KEY, parent_id bigint);"
        ))
        .await
        .unwrap_or_else(|e| panic!("Create table failed: {}", e));

    (parent, child)
}

pub async fn column_name(client: &Client, table: &str) -> Vec<String> {
    client
        .query(
            "SELECT column_name FROM information_schema.columns
                         WHERE table_schema = 'public' AND table_name = $1
                         ORDER BY ordinal_position",
            &[&table],
        )
        .await
        .unwrap()
        .iter()
        .map(|r| r.get(0))
        .collect()
}

pub async fn is_nullable(client: &Client, table: &str, column: &str) -> bool {
    let row = client
        .query_one(
            "SELECT is_nullable FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = $1 AND column_name = $2",
            &[&table, &column],
        )
        .await
        .unwrap_or_else(|e| panic!("is_nullable({table}.{column}): {e}"));
    let v: &str = row.get(0);
    v == "YES"
}

pub async fn index_is_valid(client: &Client, index_name: &str) -> bool {
    let rows = client
        .query(
            "SELECT i.indisvalid
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            JOIN pg_index i ON i.indexrelid = c.id
            WHERE n.nspname = 'public' AND c.relname = $1",
            &[&index_name],
        )
        .await
        .unwrap();

    rows.len() == 1 && rows[0].get::<_, bool>(0)
}

pub async fn constraint_state(client: &Client, table: &str, name: &str) -> Option<bool> {
    let rows = client
        .query(
            "SELECT convalidated FROM pg_constraint
            WHERE conrelid = $1::regclss AND conname = $2",
            &[&table, &name],
        )
        .await
        .unwrap();
    rows.first().map(|row| row.get(0))
}

pub async fn column_is_not_null_in_pg_attribute(
    client: &Client,
    table: &str,
    column: &str,
) -> bool {
    let row = client
        .query_one(
            "SELECT attnotnull
        FROM pg_attribute WHERE attrelid = $1::regclass AND attname = &2 AND attnum > 0",
            &[&table, &column],
        )
        .await
        .unwrap_or_else(|e| panic!("attnotnull{table}.{column}:{e}"));
    row.get(0)
}
