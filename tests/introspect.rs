mod common;

#[tokio::test]
async fn introspect_returns_all_columns_in_order() {
    let db = common::TestDB::start().await;
    let table = common::create_test_table(&db.client, "intro_cols").await;

    let info = pgmorph::db::introspect_table(&db.client, "public", &table)
        .await
        .expect("introspection table introspector");

    let names: Vec<&str> = info.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["id", "amount", "status", "created_at"]);
}

#[tokio::test]
async fn introspect_reports_nullable_correctly() {
    let db = common::TestDB::start().await;
    let table = common::create_test_table(&db.client, "intro_cols").await;

    let info = pgmorph::db::introspect_table(&db.client, "public", &table)
        .await
        .expect("introspection table introspector");

    let amount = info.columns.iter().find(|c| c.name == "amount").unwrap();
    let status = info.columns.iter().find(|c| c.name == "status").unwrap();

    assert!(!amount.nullable, "amount is not null");
    assert!(status.nullable, "status is nullable");
}

#[tokio::test]
async fn introspect_reports_default_correctly() {
    let db = common::TestDB::start().await;
    let table = common::create_test_table(&db.client, "intro_cols").await;

    let info = pgmorph::db::introspect_table(&db.client, "public", &table)
        .await
        .expect("introspection table introspector");

    let created_at = info
        .columns
        .iter()
        .find(|c| c.name == "created_at")
        .unwrap();

    assert!(created_at.default_value.is_some(), "amount is not null");
}

#[tokio::test]
async fn introspect_identifies_primary_key() {
    let db = common::TestDB::start().await;
    let table = common::create_test_table(&db.client, "intro_cols").await;

    let info = pgmorph::db::introspect_table(&db.client, "public", &table)
        .await
        .expect("introspection table introspector");

    assert_eq!(info.primary_key, vec!["id"], "primary");
}

#[tokio::test]
async fn introspect_identifies_primary_indexes() {
    let db = common::TestDB::start().await;
    let table = common::create_test_table(&db.client, "intro_cols").await;

    let info = pgmorph::db::introspect_table(&db.client, "public", &table)
        .await
        .expect("introspection table introspector");

    let pkey = info.indexes.iter().find(|i| i.is_primary);

    assert!(pkey.is_some(), "primary index is not null");

    let pkey = pkey.unwrap();
    assert!(pkey.is_unique, "primary index is not unique");
    assert!(pkey.columns.contains(&"id".to_string()));
}

#[tokio::test]
async fn introspect_reports_seconday_index_after_creation() {
    let db = common::TestDB::start().await;
    let table = common::create_test_table(&db.client, "intro_cols").await;

    db.client
        .batch_execute(&format!(
            "CREATE INDEX {table}_status_idx ON {table} (status)"
        ))
        .await
        .expect("create seconday index");

    let info = pgmorph::db::introspect_table(&db.client, "public", &table)
        .await
        .expect("introspection table introspector");

    let sec = info
        .indexes
        .iter()
        .find(|i| i.name == format!("{table}_status_idx"));

    assert!(sec.is_some(), "secondary index is not null");
    let sec = sec.unwrap();
    assert!(!sec.is_primary, "secondary index is not primary");
    assert!(sec.columns.contains(&"status".to_string()));
}
