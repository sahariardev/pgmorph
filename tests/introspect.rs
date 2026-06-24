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
