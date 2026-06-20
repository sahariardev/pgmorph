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
    pub async fn new() -> TestDB {
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
}
