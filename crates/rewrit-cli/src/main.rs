mod app;
mod commands;

use app::{Cli, Commands, SchemaCommand};
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
        Commands::Init { template } => {
            ensure_supported(
                "template",
                &template,
                &["command-to-command", "laravel-to-encore", "django-to-rust"],
            )?;
            commands::init::run(template)?
        }
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
            ensure_supported("discover format", &format, &["terminal", "json"])?;
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
                _ => return unsupported("run mode", &mode, &["mirror"]),
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
        Commands::Schema { command } => {
            match &command {
                SchemaCommand::Export { kind } => {
                    ensure_supported(
                        "schema kind",
                        kind,
                        &["report", "contract", "observation", "event"],
                    )?;
                }
            }
            commands::schema::run(command)?
        }
        Commands::Report { command } => commands::report::run(command)?,
    };

    Ok(exit_code)
}

#[derive(Debug, Error)]
enum CliError {
    #[error("{0}")]
    Engine(#[from] EngineError),
    #[error("unsupported {what}: {value}. expected one of: {expected}")]
    UnsupportedFeature {
        what: &'static str,
        value: String,
        expected: String,
    },
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl CliError {
    fn exit_code(&self) -> i32 {
        match self {
            Self::Engine(error) => error.exit_code(),
            Self::UnsupportedFeature { .. } => 9,
            Self::Json(_) => 70,
            Self::Io(_) => 7,
        }
    }
}

fn ensure_supported(
    what: &'static str,
    value: &str,
    supported: &[&'static str],
) -> Result<(), CliError> {
    if supported.contains(&value) {
        Ok(())
    } else {
        unsupported(what, value, supported)
    }
}

fn unsupported<T>(
    what: &'static str,
    value: &str,
    supported: &[&'static str],
) -> Result<T, CliError> {
    Err(CliError::UnsupportedFeature {
        what,
        value: value.to_string(),
        expected: supported.join(", "),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_feature_uses_feature_exit_code() {
        let error = ensure_supported("run mode", "baseline", &["mirror"]).expect_err("error");
        assert_eq!(error.exit_code(), 9);
        assert_eq!(
            error.to_string(),
            "unsupported run mode: baseline. expected one of: mirror"
        );
    }
}
