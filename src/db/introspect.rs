use tokio_postgres::Client;

#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
}

#[derive(Debug, Clone)]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub references_schemas: String,
    pub references_table: String,
    pub references_columns: String,
}

#[derive(Debug, Clone)]
pub struct TableInfo {
    pub schema: String,
    pub name: String,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKey>,
    pub indexes: Vec<Index>,
}

#[derive(Debug)]
pub enum IntrospectError {
    TableNotFound { schema: String, table: String },
    Database(tokio_postgres::Error),
}

impl std::fmt::Display for IntrospectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntrospectError::TableNotFound { schema, table } => {
                write!(f, "Table {} not found", schema)
            }
            IntrospectError::Database(err) => {
                write!(f, "Database error: {}", err)
            }
        }
    }
}

impl std::error::Error for IntrospectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IntrospectError::Database(err) => Some(err),
            _ => None,
        }
    }
}

impl From<tokio_postgres::Error> for IntrospectError {
    fn from(e: tokio_postgres::Error) -> Self {
        Self::Database(e)
    }
}

pub async fn introspect_table(
    client: &Client,
    schema: &str,
    table: &str,
) -> Result<TableInfo, IntrospectError> {
    if !table_exists(client, table, schema).await? {
        return Err(IntrospectError::TableNotFound {
            schema: schema.to_string(),
            table: table.to_string(),
        });
    }

    let columns = fetch_columns(client, table, schema).await?;
    let primary_keys = fetch_primary_key(client, table, schema).await?;

    //fetch fk, index
    //return tableinfo
    todo!("need to implement this")
}

async fn table_exists(client: &Client, table: &str, schema: &str) -> Result<bool, IntrospectError> {
    let row = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
                                        WHERE table_name = $1 AND table_schema = $2)",
            &[&table, &schema],
        )
        .await?;
    Ok(row.get(0))
}

async fn fetch_columns(
    client: &Client,
    table: &str,
    schema: &str,
) -> Result<Vec<Column>, IntrospectError> {
    let rows = client
        .query(
            "SELECT column_name, data_type, is_nullable, column_default \
                                                 FROM information_schema.columns \
                                                 WHERE table_schema = $1 AND table_name = $2 \
                                                 ORDER BY ordinal_position",
            &[&schema, &table],
        )
        .await?;
    Ok(rows
        .iter()
        .map(|row| Column {
            name: row.get(0),
            data_type: row.get(1),
            nullable: row.get::<_, String>(2) == "YES",
            default_value: row.get(3),
        })
        .collect())
}

async fn fetch_primary_key(
    client: &Client,
    table: &str,
    schema: &str,
) -> Result<Vec<String>, IntrospectError> {
    let rows = client.query("\
                SELECT kcu.column_name \
                FROM information_schema.table_constrains tc \
                JOIN information_schema.key_column_usage kcu \
                    ON (tc.constraint_schema = kcu.constraint_schema AND tc.constraint_name = kcu.constraint_name) \
                WHERE  tc.table_schema = $1 AND tc.table_name = $2 AND tc.constraint_type = 'PRIMARY_KEY' \
                ORDER BY kcu.ordinal_position \
    ", &[&schema, &table]).await?;

    Ok(rows.iter().map(|row| row.get(0)).collect())
}
