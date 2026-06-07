use std::fmt::Write as _;
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
    pub reference_schema: String,
    pub reference_table: String,
    pub references_columns: Vec<String>,
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
    let primary_key = fetch_primary_key(client, table, schema).await?;
    let foreign_keys = fetch_foreign_keys(client, table, schema).await?;
    let indexes = fetch_indexes(client, table, schema).await?;

    Ok(TableInfo {
        schema: schema.to_string(),
        name: table.to_string(),
        columns,
        primary_key,
        indexes,
        foreign_keys,
    })
}

pub fn format_table_info(info: &TableInfo) -> String {
    let mut output = String::new();

    writeln!(output, "Table: {} {}", info.schema, info.name).unwrap();
    writeln!(output).unwrap();

    writeln!(output, "Columns:").unwrap();

    if info.columns.is_empty() {
        writeln!(output, " (none)").unwrap();
    } else {
        for column in &info.columns {
            let nullable = if column.nullable { "NULL" } else { "NOT NULL" };
            let default = column.default_value.as_deref().unwrap_or("(no default)");
            writeln!(
                output,
                "    {:<16} {:<20} {:<10} default: {default}",
                column.name, column.data_type, nullable
            )
            .unwrap();
        }
    }

    writeln!(output).unwrap();
    writeln!(output, "Primary Key:").unwrap();

    if info.primary_key.is_empty() {
        writeln!(output, "  (none)").unwrap();
    } else {
        writeln!(output, "  {}", info.primary_key.join(", ")).unwrap();
    }

    writeln!(output).unwrap();

    writeln!(output, "Indexes:").unwrap();

    if info.indexes.is_empty() {
        writeln!(output, " (none)").unwrap();
    } else {
        for index in &info.indexes {
            let mut flags = Vec::new();

            if index.is_primary {
                flags.push("PRIMARY");
            }
            if index.is_unique {
                flags.push("UNIQUE");
            }
            let flag_text = if flags.is_empty() {
                String::from("NON-UNIQUE")
            } else {
                flags.join(", ")
            };

            writeln!(
                output,
                "  {} ({}) on ({})",
                index.name,
                flag_text,
                index.columns.join(", ")
            )
            .unwrap()
        }
    }

    writeln!(output).unwrap();

    writeln!(output, "Foreign keys:").unwrap();
    if info.foreign_keys.is_empty() {
        writeln!(output, " (none)").unwrap();
    } else {
        for foreign_key in &info.foreign_keys {
            writeln!(
                output,
                "    {}: ({}) -> {}.{}({})",
                foreign_key.name,
                foreign_key.columns.join(", "),
                foreign_key.reference_schema,
                foreign_key.reference_table,
                foreign_key.references_columns.join(", ")
            )
            .unwrap();
        }
    }

    output
}

async fn table_exists(client: &Client, table: &str, schema: &str) -> Result<bool, IntrospectError> {
    let row = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables 
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
            "SELECT column_name, data_type, is_nullable, column_default 
                                                 FROM information_schema.columns 
                                                 WHERE table_schema = $1 AND table_name = $2 
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
    let rows = client.query("
                SELECT kcu.column_name 
                FROM information_schema.table_constrains tc 
                JOIN information_schema.key_column_usage kcu 
                    ON (tc.constraint_schema = kcu.constraint_schema AND tc.constraint_name = kcu.constraint_name) 
                WHERE  tc.table_schema = $1 AND tc.table_name = $2 AND tc.constraint_type = 'PRIMARY_KEY' 
                ORDER BY kcu.ordinal_position 
    ", &[&schema, &table]).await?;

    Ok(rows.iter().map(|row| row.get(0)).collect())
}

async fn fetch_foreign_keys(
    client: &Client,
    table: &str,
    schema: &str,
) -> Result<Vec<ForeignKey>, IntrospectError> {
    let rows = client
        .query(
            "SELECT
                  tc.constraint_name,
                  kcu.column_name,
                  ccu.table_schema AS foreign_table_schema,
                  ccu.table_name AS foreign_table_name,
                  ccu.column_name AS foreign_column_name,
                  kcu.ordinal_position
                  FROM information_schema.table_constraints AS tc
                  JOIN information_schema.key_column_usage AS kcu
                    ON (tc.constraint_schema = kcu.constraint_schema AND tc.constraint_name = kcu.constraint_name)
                  JOIN information_schema.constraint_column_usage AS ccu
                    ON (tc.constraint_schema = ccu.constraint_schema AND tc.constraint_name = ccu.constraint_name)
                  WHERE tc.constraint_type = 'FOREIGN KEY'
                  AND tc.table_schema = $1 AND tc.table_name = $2
                  ORDER BY tc.constraint_name, kcu.ordinal_position
                ",
            &[&schema, &table],
        )
        .await?;

    let mut foreign_keys: Vec<ForeignKey> = Vec::new();

    for row in &rows {
        let name: String = row.get(0);
        let column: String = row.get(1);
        let reference_schema: String = row.get(2);
        let reference_table: String = row.get(3);
        let reference_column: String = row.get(4);

        if let Some(fk) = foreign_keys.iter_mut().find(|fk| fk.name == name) {
            fk.columns.push(column);
            fk.references_columns.push(reference_column);
        } else {
            foreign_keys.push(ForeignKey {
                name,
                columns: vec![column],
                reference_schema,
                reference_table,
                references_columns: vec![reference_column],
            })
        }
    }

    Ok(foreign_keys)
}

async fn fetch_indexes(
    client: &Client,
    table: &str,
    schema: &str,
) -> Result<Vec<Index>, IntrospectError> {
    let rows = client
        .query(
            "SELECT
            ic.relname AS index_name,
            ix.indisunique,
            ix.indisprimary,
            array_agg(a.attname ORDER BY cols.ordinality) AS columns
            FROM pg_class tc 
            JOIN pg_namespace tn ON tn.oid = tc.realnamespace 
            JOIN pg_index ix ON ix.indrelid = tc.oid
            JOIN pg_class ic ON ic.oid = ix.indexrelid 
            JOIN LATERAL unnest(ix.indkey) WITH ORDINALITY AS cols(attnum, ordinality) ON true 
            JOIN pg_attribute a ON a.attrelid = tc.oid AND a.attnum = cols.attnum 
            WHERE tn.nspname = $1 
            AND tc.realname = $2 
            AND a.attnum > 0 
            GROUP BY ic.realname, ix.indisunique, ix.indisprimary
            ORDER BY ic.realname",
            &[&schema, &table],
        )
        .await?;

    Ok(rows
        .iter()
        .map(|row| Index {
            name: row.get(0),
            columns: row.get(3),
            is_unique: row.get(1),
            is_primary: row.get(2),
        })
        .collect())
}
