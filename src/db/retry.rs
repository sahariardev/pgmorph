use tokio_postgres::Client;

pub async fn run_ddl_with_retry(client: &Client, stmt: &str) {
    //if dry run, print stm and return
    // try to execute the stmt with a timeout and retry
    // only do retry for unable to get lock
    
    todo!("Implement this")
}