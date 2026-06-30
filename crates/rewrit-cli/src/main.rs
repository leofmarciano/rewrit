mod app;
mod commands;

use app::{Cli, Commands};
use clap::Parser;
use miette::{IntoDiagnostic, Result};
use rewrit_engine::{Engine, RunMode};
use rewrit_model::CaseId;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rewrit=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let exit_code = match cli.command {
        Commands::Init { template } => commands::init::run(template).into_diagnostic()?,
        Commands::Doctor { manifest } => {
            let engine = Engine::from_manifest_path(manifest).into_diagnostic()?;
            let report = engine.doctor().await.into_diagnostic()?;
            print!("{}", rewrit_report::terminal::render(&report));
            report.summary.exit_code
        }
        Commands::Discover {
            manifest,
            runtime,
            format,
        } => {
            let engine = Engine::from_manifest_path(manifest).into_diagnostic()?;
            let runtime = runtime.map(rewrit_model::RuntimeId::new);
            let cases = engine.discover(runtime.as_ref()).await.into_diagnostic()?;
            commands::discover::print_cases(&cases, &format).into_diagnostic()?;
            0
        }
        Commands::Capture { manifest, runtime } => {
            let engine = Engine::from_manifest_path(manifest).into_diagnostic()?;
            let report = engine
                .capture(&rewrit_model::RuntimeId::new(runtime))
                .await
                .into_diagnostic()?;
            print!("{}", rewrit_report::terminal::render(&report));
            report.summary.exit_code
        }
        Commands::Verify {
            manifest,
            runtime,
            contracts,
        } => {
            let engine = Engine::from_manifest_path(manifest).into_diagnostic()?;
            if !contracts.is_empty() {
                tracing::info!(contracts = ?contracts, "contract selection is accepted by the CLI and resolved by manifest-backed runtimes in this MVP");
            }
            let runtime =
                runtime.unwrap_or_else(|| engine.manifest().project.candidate.to_string());
            let report = engine
                .verify(&rewrit_model::RuntimeId::new(runtime))
                .await
                .into_diagnostic()?;
            print!("{}", rewrit_report::terminal::render(&report));
            report.summary.exit_code
        }
        Commands::Run { manifest, mode } => {
            let engine = Engine::from_manifest_path(manifest).into_diagnostic()?;
            let mode = match mode.as_str() {
                "mirror" => RunMode::Mirror,
                other => {
                    eprintln!("unsupported run mode: {other}");
                    std::process::exit(9);
                }
            };
            let report = engine.run(mode).await.into_diagnostic()?;
            print!("{}", rewrit_report::terminal::render(&report));
            report.summary.exit_code
        }
        Commands::Audit { manifest } => {
            let engine = Engine::from_manifest_path(manifest).into_diagnostic()?;
            let report = engine.audit().await.into_diagnostic()?;
            print!("{}", rewrit_report::terminal::render(&report));
            report.summary.exit_code
        }
        Commands::Explain { manifest, case_id } => {
            let engine = Engine::from_manifest_path(manifest).into_diagnostic()?;
            let result = engine
                .explain(&CaseId::new(case_id))
                .await
                .into_diagnostic()?;
            commands::explain::print(result);
            0
        }
        Commands::Schema { command } => commands::schema::run(command).into_diagnostic()?,
        Commands::Report { command } => commands::report::run(command).into_diagnostic()?,
    };

    std::process::exit(exit_code);
}
