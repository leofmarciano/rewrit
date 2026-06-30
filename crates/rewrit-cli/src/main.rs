mod app;
mod commands;

use app::{Cli, Commands};
use clap::Parser;
use rewrit_engine::{Engine, EngineError, RunMode};
use rewrit_model::CaseId;
use thiserror::Error;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rewrit=info".into()),
        )
        .init();

    let cli = Cli::parse();
    match run(cli).await {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(error.exit_code());
        }
    }
}

async fn run(cli: Cli) -> Result<i32, CliError> {
    let exit_code = match cli.command {
        Commands::Init { template } => commands::init::run(template)?,
        Commands::Doctor { manifest } => {
            let engine = Engine::from_manifest_path(manifest)?;
            let report = engine.doctor().await?;
            print!("{}", rewrit_report::terminal::render(&report));
            report.summary.exit_code
        }
        Commands::Discover {
            manifest,
            runtime,
            format,
        } => {
            let engine = Engine::from_manifest_path(manifest)?;
            let runtime = runtime.map(rewrit_model::RuntimeId::new);
            let cases = engine.discover(runtime.as_ref()).await?;
            commands::discover::print_cases(&cases, &format)?;
            0
        }
        Commands::Capture { manifest, runtime } => {
            let engine = Engine::from_manifest_path(manifest)?;
            let report = engine
                .capture(&rewrit_model::RuntimeId::new(runtime))
                .await?;
            print!("{}", rewrit_report::terminal::render(&report));
            report.summary.exit_code
        }
        Commands::Verify {
            manifest,
            runtime,
            contracts,
        } => {
            let engine = Engine::from_manifest_path(manifest)?;
            let report = if contracts.is_empty() {
                let runtime =
                    runtime.unwrap_or_else(|| engine.manifest().project.candidate.to_string());
                engine
                    .verify(&rewrit_model::RuntimeId::new(runtime))
                    .await?
            } else {
                engine.verify_contracts(&contracts).await?
            };
            print!("{}", rewrit_report::terminal::render(&report));
            report.summary.exit_code
        }
        Commands::Run { manifest, mode } => {
            let engine = Engine::from_manifest_path(manifest)?;
            let mode = match mode.as_str() {
                "mirror" => RunMode::Mirror,
                other => {
                    eprintln!("unsupported run mode: {other}");
                    std::process::exit(9);
                }
            };
            let report = engine.run(mode).await?;
            print!("{}", rewrit_report::terminal::render(&report));
            report.summary.exit_code
        }
        Commands::Audit { manifest } => {
            let engine = Engine::from_manifest_path(manifest)?;
            let report = engine.audit().await?;
            print!("{}", rewrit_report::terminal::render(&report));
            report.summary.exit_code
        }
        Commands::Explain { manifest, case_id } => {
            let engine = Engine::from_manifest_path(manifest)?;
            let result = engine.explain(&CaseId::new(case_id)).await?;
            commands::explain::print(result);
            0
        }
        Commands::Schema { command } => commands::schema::run(command)?,
        Commands::Report { command } => commands::report::run(command)?,
    };

    Ok(exit_code)
}

#[derive(Debug, Error)]
enum CliError {
    #[error("{0}")]
    Engine(#[from] EngineError),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl CliError {
    fn exit_code(&self) -> i32 {
        match self {
            Self::Engine(error) => error.exit_code(),
            Self::Json(_) => 70,
            Self::Io(_) => 7,
        }
    }
}
