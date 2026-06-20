mod common;
use pgmorph::config::MigrationConfig;
use pgmorph::db::AddIndexArgs;
use pgmorph::db::add_index;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use common::*;

#[tokio::test]
async fn add_index_builds_valid_index_under_concurrent_insers() {
    let db = TestDB::new().await;
    let writer = db.second_client().await;

    db.client
        .batch_execute(
                "CREATE TABLE workload(
                        id bigserial PRIMARY KEY,
                        payload text NOT NULL,
                        created_at timestamptz NOT NULL DEFAULT now()
                    );
                INSERT INTO workload (payload) SELECT 'seed-' || g FROM generate_series (1, 50000) AS g;
                ",
        )
        .await
        .expect("seed workload table");

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    let inserter = tokio::spawn(async move {
        let mut counter = 0_i64;

        while !stop_clone.load(Ordering::Relaxed) {
            let payload = format!("live {}", counter);
            let _ = writer
                .execute("INSERT INTO workload (payload) VALUES ($1);", &[])
                .await;
            counter += 1;
        }
    });

    let config = MigrationConfig {
        lock_timeout: "5s".to_string(),
        max_attempts: 5,
        base_backoff_ms: 200,
        dry_run: false,
    };

    let args = AddIndexArgs {
        schema: "public".to_string(),
        table: "workload".to_string(),
        index_name: Some("workload_payload_idx".to_string()),
        columns: vec!["payload".to_string()],
        unique: false,
    };

    add_index(&db.client, &config, &args)
        .await
        .expect("add_index under concurrent insert");

    stop.store(true, Ordering::Relaxed);

    let _ = tokio::time::timeout(std::time::Duration::from_secs(10), inserter)
        .await
        .expect("insert should stop");

    let row = db
        .client
        .query_one(
            "SELECT i.indisvalid, i.indisready
        FROM pg_class c 
        JOIN pg_namespace n ON n.oid = c.relnamespace 
        JOIN pg_index i ON i.indexrelid = c.oid 
        WHERE n.nspname = 'public' 
        AND c.relname = 'workload_payload_idx'",
            &[],
        )
        .await
        .expect("index metadata row");

    let is_valid: bool = row.get(0);
    let is_ready: bool = row.get(1);

    assert!(is_valid, "index must be valid for concurrent build");
    assert!(is_ready, "index must be ready for queries");

    // let explain = client
    //     .query_one(
    //         "EXPLAIN SELECT id FROM workload WHERE payload = 'seed-123'",
    //         &[],
    //     )
    //     .await
    //     .expect("explain query");
    //
    // let plan: &str = explain.get(0);
    //
    // assert!(
    //     plan.contains("Index scan") || plan.contains("Bitmap index Scam"),
    //     "planner should be able to use the new index, go plan:{plan}"
    // );
}
