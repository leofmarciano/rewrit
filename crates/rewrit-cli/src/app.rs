use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "rewrit", version, about = "Parity engine for observable rewrite contracts")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init {
        #[arg(long, default_value = "command-to-command")]
        template: String,
    },
    Doctor {
        #[arg(long, default_value = "rewrit.toml")]
        manifest: PathBuf,
    },
    Discover {
        #[arg(long, default_value = "rewrit.toml")]
        manifest: PathBuf,
        #[arg(long)]
        runtime: Option<String>,
        #[arg(long, default_value = "terminal")]
        format: String,
    },
    Capture {
        #[arg(long, default_value = "rewrit.toml")]
        manifest: PathBuf,
        #[arg(long)]
        runtime: String,
    },
    Verify {
        #[arg(long, default_value = "rewrit.toml")]
        manifest: PathBuf,
        #[arg(long)]
        runtime: Option<String>,
        #[arg(long = "contracts")]
        contracts: Vec<String>,
    },
    Run {
        #[arg(long, default_value = "rewrit.toml")]
        manifest: PathBuf,
        #[arg(long, default_value = "mirror")]
        mode: String,
    },
    Audit {
        #[arg(long, default_value = "rewrit.toml")]
        manifest: PathBuf,
    },
    Explain {
        #[arg(long, default_value = "rewrit.toml")]
        manifest: PathBuf,
        case_id: String,
    },
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    Report {
        #[command(subcommand)]
        command: ReportCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum SchemaCommand {
    Export {
        #[arg(long, default_value = "report")]
        kind: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ReportCommand {
    Open {
        #[arg(long, default_value = ".rewrit/reports/latest.json")]
        path: PathBuf,
    },
}

