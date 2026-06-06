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
    Database (tokio_postgres::Error)
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
    //check table exist
    //fetch columns, pk, fk, index
    //return tableinfo
    todo!("need to implement this")
}