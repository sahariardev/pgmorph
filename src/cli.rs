use clap::{Parser, Subcommand, ValueEnum};
const DEFAULT_DATABASE_URL: &str =
    "host=localhost port=5433 user=pgmorph password=pgmorph dbname=pgmorph";

#[derive(Parser, Debug)]
#[command(
    name = "pgmorph",
    about = "Zero-downtime schema migration for postgresql"
)]
pub struct Cli {
    #[arg(long, env = "DATABASE_URL", default_value = DEFAULT_DATABASE_URL, global = true)]
    pub database_url: String,

    #[arg(long, global = true)]
    pub dry_run: bool,

    #[arg(long, default_value = "5s", global = true)]
    pub lock_timeout: String,

    #[arg(long, default_value = "5", global = true)]
    pub max_attempts: u32,

    #[command(subcommand)]
    pub command: Option<Command>,
}
#[derive(Debug, Clone, ValueEnum)]
pub enum OnDeleteAction {
    Restrict,
    SetNull,
    Cascade,
    NoAction,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Introspect {
        #[arg(long)]
        table: String,

        #[arg(long, default_value = "public")]
        schema: String,
    },

    AddColumn {
        #[arg(long)]
        table: String,

        #[arg(long, default_value = "public")]
        schema: String,

        #[arg(long)]
        column: String,

        #[arg(long, alias = "type")]
        data_type: String,

        #[arg(long)]
        default: Option<String>,

        #[arg(long)]
        not_null: bool,
    },

    AddCheck {
        #[arg(long)]
        table: String,

        #[arg(long, default_value = "public")]
        schema: String,

        #[arg(long)]
        constraint_name: String,

        #[arg(long)]
        rule: String,
    },

    AddForeignKey {
        #[arg(long)]
        table: String,

        #[arg(long, default_value = "public")]
        schema: String,

        #[arg(long)]
        constraint_name: Option<String>,

        #[arg(long)]
        column: String,

        #[arg(long)]
        foreign_table_name: String,

        #[arg(long)]
        foreign_column_name: String,

        on_delete: OnDeleteAction,
    },
    SetNotNull {
        #[arg(long)]
        table: String,

        #[arg(long, default_value = "public")]
        schema: String,

        #[arg(long)]
        column: String,
    },
}
