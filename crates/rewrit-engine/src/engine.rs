use crate::discovery::manifest::{
    AdapterProtocolInput, AdapterProtocolOutput, Manifest as ManifestConfig, NetworkMode,
    PolicyConfig, ReportConfig, RuntimeConfig,
};
use crate::runner::process::{ProcessError, ProcessRunner, RuntimeProcess};
use crate::runner::timeout::millis;
use crate::store::baseline;
use crate::store::filesystem::RewritStore;
use rewrit_adapter_http::HttpAdapterError;
use rewrit_core::compare::Comparator;
use rewrit_core::normalize::http::HttpHeaderNormalizer;
use rewrit_core::normalize::path::PathNormalizer;
use rewrit_core::normalize::regex::RegexNormalizer;
use rewrit_core::normalize::time::timestamp_normalizer;
use rewrit_core::normalize::{NormalizationPipeline, NormalizeContext, Normalizer};
use rewrit_core::policy::{Policy, PolicyEngine, Waiver, WaiverSet};
use rewrit_core::{CompareContext, StrictComparator};
use rewrit_model::{
    CanonicalValue, CapturedText, Case, CaseId, CaseStatus, Contract, ContractRef, Divergence,
    DivergenceKind, MinimalReproduction, Observation, Report, ReportSummary, RuntimeId, Severity,
    SourceLocation, SuiteId, SuiteSummary,
};
use rewrit_protocol::{
    decode_events, encode_request_line, AdapterCommand, AdapterEvent, AdapterRequest,
};
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use thiserror::Error;
use tokio::process::{Child, Command};
use uuid::Uuid;

pub type Manifest = ManifestConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Mirror,
}

#[derive(Debug, Clone)]
pub struct EngineOptions {
    pub manifest_path: PathBuf,
    pub root: PathBuf,
}

impl EngineOptions {
    #[must_use]
    pub fn new(manifest_path: impl Into<PathBuf>) -> Self {
        let manifest_path = manifest_path.into();
        let root = manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        Self {
            manifest_path,
            root,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExplainResult {
    pub case_id: CaseId,
    pub divergences: Vec<Divergence>,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("failed to read manifest {path}: {source}")]
    ReadManifest {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse manifest {path}: {source}")]
    ParseManifest {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("runtime not found: {0}")]
    RuntimeNotFound(RuntimeId),
    #[error("runtime {runtime_id} failed: {source}")]
    RuntimeFailed {
        runtime_id: RuntimeId,
        #[source]
        source: ProcessError,
    },
    #[error("global {command} timeout after {timeout_ms}ms")]
    GlobalTimeout {
        command: &'static str,
        timeout_ms: u64,
    },
    #[error("adapter protocol error for runtime {runtime_id}: {source}")]
    Protocol {
        runtime_id: RuntimeId,
        #[source]
        source: rewrit_protocol::ProtocolError,
    },
    #[error("failed to encode adapter request for runtime {runtime_id}: {source}")]
    EncodeProtocolRequest {
        runtime_id: RuntimeId,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to read adapter protocol output for runtime {runtime_id} at {path}: {source}")]
    ReadProtocolOutput {
        runtime_id: RuntimeId,
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read contract {path}: {source}")]
    ReadContract {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse contract {path}: {message}")]
    ParseContract { path: String, message: String },
    #[error("http adapter failed for runtime {runtime_id}: {source}")]
    HttpAdapter {
        runtime_id: RuntimeId,
        #[source]
        source: HttpAdapterError,
    },
    #[error("failed to start HTTP server for runtime {runtime_id}: {source}")]
    StartServer {
        runtime_id: RuntimeId,
        #[source]
        source: std::io::Error,
    },
    #[error("baseline error: {0}")]
    Baseline(#[from] baseline::BaselineError),
    #[error("report error: {0}")]
    Report(#[from] rewrit_report::ReportError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl EngineError {
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::ReadManifest { .. }
            | Self::ParseManifest { .. }
            | Self::InvalidManifest(_)
            | Self::ReadContract { .. }
            | Self::ParseContract { .. } => 2,
            Self::Protocol { .. } | Self::ReadProtocolOutput { .. } | Self::HttpAdapter { .. } => 4,
            Self::RuntimeNotFound(_)
            | Self::RuntimeFailed { .. }
            | Self::StartServer { .. }
            | Self::Baseline(_) => 5,
            Self::GlobalTimeout { .. } => 6,
            Self::Report(_) => 7,
            Self::EncodeProtocolRequest { .. } | Self::Io(_) => 70,
        }
    }
}

pub struct Engine {
    pub options: EngineOptions,
    pub manifest: Manifest,
    store: RewritStore,
    runner: ProcessRunner,
}

#[derive(Debug, Clone, Default)]
struct RuntimeRun {
    cases: Vec<Case>,
    observations: Vec<Observation>,
    divergences: Vec<Divergence>,
}

#[derive(Debug, Clone)]
struct LoadedContract {
    path: PathBuf,
    contract: Contract,
}

impl Engine {
    pub fn from_manifest_path(path: impl Into<PathBuf>) -> Result<Self, EngineError> {
        let options = EngineOptions::new(path);
        let input = std::fs::read_to_string(&options.manifest_path).map_err(|source| {
            EngineError::ReadManifest {
                path: options.manifest_path.display().to_string(),
                source,
            }
        })?;
        let manifest: Manifest =
            toml::from_str(&input).map_err(|source| EngineError::ParseManifest {
                path: options.manifest_path.display().to_string(),
                source,
            })?;
        validate_manifest(&manifest)?;
        let store = RewritStore::new(
            &options.root,
            manifest.project.baselines_dir.as_deref(),
            manifest.project.reports_dir.as_deref(),
        );
        store.ensure()?;
        let runner = ProcessRunner::new(manifest.runner.clone(), &manifest.security);
        Ok(Self {
            options,
            manifest,
            store,
            runner,
        })
    }

    #[must_use]
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub async fn doctor(&self) -> Result<Report, EngineError> {
        self.with_global_timeout("doctor", self.doctor_inner())
            .await
    }

    async fn doctor_inner(&self) -> Result<Report, EngineError> {
        let mut divergences = Vec::new();
        for runtime_id in self.manifest.runtimes.keys() {
            if let Err(error) = self.run_runtime(runtime_id, AdapterCommand::Doctor).await {
                divergences.push(runtime_divergence(
                    runtime_id,
                    DivergenceKind::AdapterError,
                    format!("doctor failed for runtime {runtime_id}: {error}"),
                ));
            }
        }
        Ok(self.report_from_divergences("doctor", Vec::new(), Vec::new(), divergences))
    }

    pub async fn discover(&self, runtime_id: Option<&RuntimeId>) -> Result<Vec<Case>, EngineError> {
        self.with_global_timeout("discover", self.discover_inner(runtime_id))
            .await
    }

    async fn discover_inner(
        &self,
        runtime_id: Option<&RuntimeId>,
    ) -> Result<Vec<Case>, EngineError> {
        let runtime_ids: Vec<RuntimeId> = match runtime_id {
            Some(runtime_id) => vec![runtime_id.clone()],
            None => self.manifest.runtimes.keys().cloned().collect(),
        };

        let mut cases = Vec::new();
        for runtime_id in runtime_ids {
            cases.extend(
                self.run_runtime(&runtime_id, AdapterCommand::Discover)
                    .await?
                    .cases,
            );
        }
        Ok(cases)
    }

    pub async fn run(&self, mode: RunMode) -> Result<Report, EngineError> {
        self.with_global_timeout("run", self.run_inner(mode)).await
    }

    async fn run_inner(&self, _mode: RunMode) -> Result<Report, EngineError> {
        let reference = self
            .run_runtime(&self.manifest.project.reference, AdapterCommand::Run)
            .await?;
        let candidate = self
            .run_runtime(&self.manifest.project.candidate, AdapterCommand::Run)
            .await?;
        let report = self.compare_runs(reference, candidate);
        self.write_configured_reports(&report)?;
        Ok(report)
    }

    pub async fn capture(&self, runtime_id: &RuntimeId) -> Result<Report, EngineError> {
        self.with_global_timeout("capture", self.capture_inner(runtime_id))
            .await
    }

    async fn capture_inner(&self, runtime_id: &RuntimeId) -> Result<Report, EngineError> {
        let run = self.run_runtime(runtime_id, AdapterCommand::Run).await?;
        let _baseline_lock = self
            .store
            .acquire_lock(&format!("baseline-{}", runtime_id.as_str()))?;
        baseline::write_current(&self.store, runtime_id, &run.observations)?;
        let report =
            self.report_from_divergences("capture", run.cases, run.observations, Vec::new());
        self.write_configured_reports(&report)?;
        Ok(report)
    }

    pub async fn verify(&self, runtime_id: &RuntimeId) -> Result<Report, EngineError> {
        self.with_global_timeout("verify", self.verify_inner(runtime_id))
            .await
    }

    async fn verify_inner(&self, runtime_id: &RuntimeId) -> Result<Report, EngineError> {
        let baseline_runtime = &self.manifest.project.reference;
        let baseline_observations = baseline::read_current(&self.store, baseline_runtime)?;
        let reference = RuntimeRun {
            cases: Vec::new(),
            observations: baseline_observations,
            divergences: Vec::new(),
        };
        let candidate = self.run_runtime(runtime_id, AdapterCommand::Run).await?;
        let report = self.compare_runs(reference, candidate);
        self.write_configured_reports(&report)?;
        Ok(report)
    }

    pub async fn verify_contracts(&self, contract_paths: &[String]) -> Result<Report, EngineError> {
        self.with_global_timeout("verify", self.verify_contracts_inner(contract_paths))
            .await
    }

    async fn verify_contracts_inner(
        &self,
        contract_paths: &[String],
    ) -> Result<Report, EngineError> {
        let reference_runtime = self
            .manifest
            .runtimes
            .get(&self.manifest.project.reference)
            .ok_or_else(|| EngineError::RuntimeNotFound(self.manifest.project.reference.clone()))?;
        let candidate_runtime = self
            .manifest
            .runtimes
            .get(&self.manifest.project.candidate)
            .ok_or_else(|| EngineError::RuntimeNotFound(self.manifest.project.candidate.clone()))?;
        let contracts = self.load_contracts(Some(contract_paths))?;
        let contract_case_ids = contracts
            .iter()
            .map(|loaded| loaded.contract.id.clone())
            .collect::<Vec<_>>();
        let reference = if reference_runtime.adapter.starts_with("http") {
            self.run_http_runtime(
                &self.manifest.project.reference,
                reference_runtime,
                Some(&contracts),
            )
            .await?
        } else {
            self.run_runtime_with_cases(
                &self.manifest.project.reference,
                AdapterCommand::Run,
                contract_case_ids.clone(),
            )
            .await?
        };
        let candidate = if candidate_runtime.adapter.starts_with("http") {
            self.run_http_runtime(
                &self.manifest.project.candidate,
                candidate_runtime,
                Some(&contracts),
            )
            .await?
        } else {
            self.run_runtime_with_cases(
                &self.manifest.project.candidate,
                AdapterCommand::Run,
                contract_case_ids,
            )
            .await?
        };
        let mut reference = reference;
        let mut candidate = candidate;
        if !reference_runtime.adapter.starts_with("http") {
            apply_contracts_to_command_run(
                &contracts,
                &self.manifest.project.reference,
                &mut reference,
                DivergenceKind::MissingReferenceCase,
            );
        }
        if !candidate_runtime.adapter.starts_with("http") {
            apply_contracts_to_command_run(
                &contracts,
                &self.manifest.project.candidate,
                &mut candidate,
                DivergenceKind::MissingCandidateCase,
            );
        }
        let report = self.compare_runs(reference, candidate);
        self.write_configured_reports(&report)?;
        Ok(report)
    }

    pub async fn audit(&self) -> Result<Report, EngineError> {
        self.with_global_timeout("audit", self.audit_inner()).await
    }

    async fn audit_inner(&self) -> Result<Report, EngineError> {
        let reference = self
            .run_runtime(&self.manifest.project.reference, AdapterCommand::Discover)
            .await?;
        let candidate = self
            .run_runtime(&self.manifest.project.candidate, AdapterCommand::Discover)
            .await?;
        let suite_by_case = suite_map(&reference.cases, &candidate.cases);
        let all_ids = audit_case_ids(&reference, &candidate);
        let mut divergences = self.audit_runs(&reference, &candidate);
        attach_suites_to_divergences(&mut divergences, &suite_by_case);
        let mut report = self.report_from_divergences(
            "audit",
            reference.cases,
            candidate.observations,
            divergences,
        );
        report.suites = suite_summaries(&suite_by_case, &all_ids, &report.divergences);
        self.write_configured_reports(&report)?;
        Ok(report)
    }

    pub async fn explain(&self, case_id: &CaseId) -> Result<ExplainResult, EngineError> {
        self.with_global_timeout("explain", self.explain_inner(case_id))
            .await
    }

    async fn explain_inner(&self, case_id: &CaseId) -> Result<ExplainResult, EngineError> {
        let report = self.run_inner(RunMode::Mirror).await?;
        let divergences = report
            .divergences
            .into_iter()
            .filter(|divergence| divergence.case_id == *case_id)
            .collect();
        Ok(ExplainResult {
            case_id: case_id.clone(),
            divergences,
        })
    }

    async fn with_global_timeout<T, F>(
        &self,
        command: &'static str,
        future: F,
    ) -> Result<T, EngineError>
    where
        F: Future<Output = Result<T, EngineError>>,
    {
        let Some(timeout_ms) = self.manifest.runner.global_timeout_ms else {
            return future.await;
        };

        match tokio::time::timeout(millis(timeout_ms), future).await {
            Ok(result) => result,
            Err(_) => Err(EngineError::GlobalTimeout {
                command,
                timeout_ms,
            }),
        }
    }

    async fn run_runtime(
        &self,
        runtime_id: &RuntimeId,
        command: AdapterCommand,
    ) -> Result<RuntimeRun, EngineError> {
        self.run_runtime_with_cases(runtime_id, command, Vec::new())
            .await
    }

    async fn run_runtime_with_cases(
        &self,
        runtime_id: &RuntimeId,
        command: AdapterCommand,
        cases: Vec<CaseId>,
    ) -> Result<RuntimeRun, EngineError> {
        let runtime = self
            .manifest
            .runtimes
            .get(runtime_id)
            .ok_or_else(|| EngineError::RuntimeNotFound(runtime_id.clone()))?;

        if !is_command_protocol_adapter(&runtime.adapter) {
            if runtime.adapter == "http" || runtime.adapter.starts_with("http:") {
                return self.run_http_runtime(runtime_id, runtime, None).await;
            }
            return Ok(RuntimeRun::default());
        }

        self.run_command_runtime(runtime_id, runtime, command, cases)
            .await
    }

    async fn run_command_runtime(
        &self,
        runtime_id: &RuntimeId,
        runtime: &RuntimeConfig,
        command: AdapterCommand,
        cases: Vec<CaseId>,
    ) -> Result<RuntimeRun, EngineError> {
        let mut process = self.runner.from_runtime(&self.options.root, runtime);
        let temp_dir = absolute_path(
            self.store
                .create_temp_dir(&format!("runtime-{runtime_id}"))?,
        )?;
        process.temp_dir = Some(temp_dir);
        let protocol_output_path =
            self.prepare_command_protocol(&mut process, runtime_id, runtime, command, cases)?;
        let output =
            self.runner
                .run(&process)
                .await
                .map_err(|source| EngineError::RuntimeFailed {
                    runtime_id: runtime_id.clone(),
                    source,
                })?;

        if output.timed_out {
            let observation = Observation {
                case_id: CaseId::new(format!("{runtime_id}.timeout")),
                runtime_id: runtime_id.clone(),
                status: CaseStatus::TimedOut,
                value: None,
                error: None,
                stdout: CapturedText {
                    text: output.stdout,
                    truncated: output.stdout_truncated,
                },
                stderr: CapturedText {
                    text: output.stderr,
                    truncated: output.stderr_truncated,
                },
                exit_code: None,
                duration_ms: runtime.timeout_ms.unwrap_or(30_000),
                effects: Vec::new(),
                artifacts: Vec::new(),
                metadata: BTreeMap::new(),
            };
            return Ok(RuntimeRun {
                cases: Vec::new(),
                observations: vec![observation],
                divergences: Vec::new(),
            });
        }

        let protocol_output = self.protocol_output(runtime_id, &output, protocol_output_path)?;
        let events = decode_events(&protocol_output).map_err(|source| EngineError::Protocol {
            runtime_id: runtime_id.clone(),
            source,
        })?;

        let mut run = RuntimeRun::default();
        for event in events {
            match event {
                AdapterEvent::CaseDiscovered { case, .. } => run.cases.push(case),
                AdapterEvent::Observation { observation, .. } => run.observations.push(observation),
                AdapterEvent::AdapterError {
                    case_id, message, ..
                } => run.observations.push(Observation {
                    case_id: case_id
                        .unwrap_or_else(|| CaseId::new(format!("{runtime_id}.adapter_error"))),
                    runtime_id: runtime_id.clone(),
                    status: CaseStatus::AdapterError,
                    value: None,
                    error: None,
                    stdout: CapturedText {
                        text: output.stdout.clone(),
                        truncated: output.stdout_truncated,
                    },
                    stderr: CapturedText {
                        text: message,
                        truncated: false,
                    },
                    exit_code: output.status_code,
                    duration_ms: 0,
                    effects: Vec::new(),
                    artifacts: Vec::new(),
                    metadata: BTreeMap::new(),
                }),
                AdapterEvent::DoctorReport { .. }
                | AdapterEvent::CaseStarted { .. }
                | AdapterEvent::CaseFinished { .. } => {}
            }
        }

        if run.observations.is_empty() && output.status_code.unwrap_or(0) != 0 {
            run.observations.push(Observation {
                case_id: CaseId::new(format!("{runtime_id}.process_exit")),
                runtime_id: runtime_id.clone(),
                status: CaseStatus::InfraError,
                value: None,
                error: None,
                stdout: CapturedText {
                    text: output.stdout,
                    truncated: output.stdout_truncated,
                },
                stderr: CapturedText {
                    text: output.stderr,
                    truncated: output.stderr_truncated,
                },
                exit_code: output.status_code,
                duration_ms: 0,
                effects: Vec::new(),
                artifacts: Vec::new(),
                metadata: BTreeMap::new(),
            });
        }

        Ok(run)
    }

    fn prepare_command_protocol(
        &self,
        process: &mut RuntimeProcess,
        runtime_id: &RuntimeId,
        runtime: &RuntimeConfig,
        command: AdapterCommand,
        cases: Vec<CaseId>,
    ) -> Result<Option<PathBuf>, EngineError> {
        let Some(temp_dir) = process.temp_dir.as_ref() else {
            return Err(EngineError::InvalidManifest(format!(
                "runtime {runtime_id} has no temp dir for adapter protocol files"
            )));
        };

        process.env.insert(
            "REWRIT_RUNTIME_ID".to_string(),
            runtime_id.as_str().to_string(),
        );
        process.env.insert(
            "REWRIT_ADAPTER_COMMAND".to_string(),
            adapter_command_name(command).to_string(),
        );
        if !cases.is_empty() {
            process.env.insert(
                "REWRIT_CASES".to_string(),
                cases
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        process.env.insert(
            "REWRIT_PROTOCOL_INPUT".to_string(),
            protocol_input_name(runtime.protocol.input).to_string(),
        );
        process.env.insert(
            "REWRIT_PROTOCOL_OUTPUT".to_string(),
            protocol_output_name(runtime.protocol.output).to_string(),
        );

        if matches!(runtime.protocol.input, AdapterProtocolInput::File) {
            let request_path = temp_dir.join("adapter-request.ndjson");
            let request = AdapterRequest::new(command, runtime_id.clone(), cases);
            let encoded = encode_request_line(&request).map_err(|source| {
                EngineError::EncodeProtocolRequest {
                    runtime_id: runtime_id.clone(),
                    source,
                }
            })?;
            std::fs::write(&request_path, encoded)?;
            process.env.insert(
                "REWRIT_REQUEST_PATH".to_string(),
                request_path.display().to_string(),
            );
        }

        if matches!(runtime.protocol.output, AdapterProtocolOutput::File) {
            let events_path = temp_dir.join("adapter-events.ndjson");
            process.env.insert(
                "REWRIT_EVENTS_PATH".to_string(),
                events_path.display().to_string(),
            );
            Ok(Some(events_path))
        } else {
            Ok(None)
        }
    }

    fn protocol_output(
        &self,
        runtime_id: &RuntimeId,
        output: &crate::runner::process::ProcessOutput,
        protocol_output_path: Option<PathBuf>,
    ) -> Result<String, EngineError> {
        let Some(path) = protocol_output_path else {
            return Ok(output.stdout.clone());
        };

        match std::fs::read_to_string(&path) {
            Ok(contents) => Ok(contents),
            Err(_source) if output.status_code.unwrap_or(0) != 0 => Ok(String::new()),
            Err(source) => Err(EngineError::ReadProtocolOutput {
                runtime_id: runtime_id.clone(),
                path: path.display().to_string(),
                source,
            }),
        }
    }

    async fn run_http_runtime(
        &self,
        runtime_id: &RuntimeId,
        runtime: &RuntimeConfig,
        contracts: Option<&[LoadedContract]>,
    ) -> Result<RuntimeRun, EngineError> {
        let loaded_contracts = match contracts {
            Some(contracts) => contracts.to_vec(),
            None => self.load_contracts(None)?,
        };
        if loaded_contracts.is_empty() {
            return Ok(RuntimeRun::default());
        }
        self.validate_http_network(runtime_id, runtime)?;

        let mut server = self.start_http_server(runtime_id, runtime).await?;
        if let Some(healthcheck) = runtime
            .server
            .as_ref()
            .and_then(|server| server.healthcheck.as_ref())
        {
            rewrit_adapter_http::wait_for_healthcheck(
                healthcheck,
                std::time::Duration::from_millis(runtime.timeout_ms.unwrap_or(30_000)),
            )
            .await
            .map_err(|source| EngineError::HttpAdapter {
                runtime_id: runtime_id.clone(),
                source,
            })?;
        }

        let base_url = runtime
            .server
            .as_ref()
            .and_then(|server| server.healthcheck.as_ref())
            .map(|healthcheck| rewrit_adapter_http::base_url_from_healthcheck(healthcheck))
            .transpose()
            .map_err(|source| EngineError::HttpAdapter {
                runtime_id: runtime_id.clone(),
                source,
            })?
            .ok_or_else(|| {
                EngineError::InvalidManifest(format!(
                    "http runtime {runtime_id} requires server.healthcheck"
                ))
            })?;

        let mut cases = Vec::new();
        let mut observations = Vec::new();
        let mut divergences = Vec::new();
        for loaded in &loaded_contracts {
            cases.push(case_from_contract(loaded));
            match rewrit_adapter_http::execute_contract(
                &base_url,
                runtime_id.clone(),
                &loaded.contract,
                std::time::Duration::from_millis(runtime.timeout_ms.unwrap_or(30_000)),
            )
            .await
            {
                Ok(observation) => {
                    divergences.extend(validate_http_contract_observation(
                        &loaded.contract,
                        &observation,
                    ));
                    observations.push(observation);
                }
                Err(error) => observations.push(http_adapter_error_observation(
                    runtime_id,
                    &loaded.contract.id,
                    error,
                )),
            }
        }

        if let Some(child) = &mut server {
            let _ = child.kill().await;
        }

        Ok(RuntimeRun {
            cases,
            observations,
            divergences,
        })
    }

    fn load_contracts(
        &self,
        patterns: Option<&[String]>,
    ) -> Result<Vec<LoadedContract>, EngineError> {
        let paths = match patterns {
            Some(patterns) if !patterns.is_empty() => {
                self.contract_paths_from_patterns(patterns)?
            }
            _ => {
                let contracts_dir = self
                    .manifest
                    .project
                    .contracts_dir
                    .as_deref()
                    .unwrap_or("contracts");
                let mut paths = Vec::new();
                collect_contract_files(&self.options.root.join(contracts_dir), &mut paths)?;
                paths
            }
        };

        paths
            .into_iter()
            .map(|path| {
                let input =
                    std::fs::read_to_string(&path).map_err(|source| EngineError::ReadContract {
                        path: path.display().to_string(),
                        source,
                    })?;
                let extension = path
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or("");
                let contract = match extension {
                    "yaml" | "yml" => {
                        serde_yaml::from_str::<Contract>(&input).map_err(|source| {
                            EngineError::ParseContract {
                                path: path.display().to_string(),
                                message: source.to_string(),
                            }
                        })?
                    }
                    _ => serde_json::from_str::<Contract>(&input).map_err(|source| {
                        EngineError::ParseContract {
                            path: path.display().to_string(),
                            message: source.to_string(),
                        }
                    })?,
                };
                Ok(LoadedContract { path, contract })
            })
            .collect()
    }

    fn contract_paths_from_patterns(
        &self,
        patterns: &[String],
    ) -> Result<Vec<PathBuf>, EngineError> {
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in patterns {
            builder.add(globset::Glob::new(pattern).map_err(|error| {
                EngineError::InvalidManifest(format!("invalid contract glob {pattern}: {error}"))
            })?);
        }
        let globset = builder.build().map_err(|error| {
            EngineError::InvalidManifest(format!("invalid contract globs: {error}"))
        })?;
        let mut all_files = Vec::new();
        collect_contract_files(&self.options.root, &mut all_files)?;
        Ok(all_files
            .into_iter()
            .filter(|path| {
                let relative = path.strip_prefix(&self.options.root).unwrap_or(path);
                globset.is_match(relative)
            })
            .collect())
    }

    async fn start_http_server(
        &self,
        runtime_id: &RuntimeId,
        runtime: &RuntimeConfig,
    ) -> Result<Option<Child>, EngineError> {
        let Some(server) = &runtime.server else {
            return Ok(None);
        };
        let Some((program, args)) = server.start.split_first() else {
            return Ok(None);
        };

        let cwd = runtime
            .cwd
            .as_ref()
            .map(|cwd| self.options.root.join(cwd))
            .unwrap_or_else(|| self.options.root.clone());
        let mut command = Command::new(program);
        let temp_dir = self
            .store
            .create_temp_dir(&format!("runtime-{runtime_id}-server"))?;
        command
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(self.manifest.runner.kill_process_tree);
        self.runner
            .apply_environment(&mut command, &runtime.env, Some(&temp_dir));

        command
            .spawn()
            .map(Some)
            .map_err(|source| EngineError::StartServer {
                runtime_id: runtime_id.clone(),
                source,
            })
    }

    fn validate_http_network(
        &self,
        runtime_id: &RuntimeId,
        runtime: &RuntimeConfig,
    ) -> Result<(), EngineError> {
        match self.manifest.security.network_mode {
            NetworkMode::Inherit => Ok(()),
            NetworkMode::Disabled => Err(EngineError::InvalidManifest(format!(
                "security.network_mode=disabled cannot run built-in HTTP runtime {runtime_id}"
            ))),
            NetworkMode::LoopbackOnly => {
                let Some(healthcheck) = runtime
                    .server
                    .as_ref()
                    .and_then(|server| server.healthcheck.as_ref())
                else {
                    return Ok(());
                };
                if is_loopback_url(healthcheck) {
                    Ok(())
                } else {
                    Err(EngineError::InvalidManifest(format!(
                        "security.network_mode=loopback_only requires HTTP runtime {runtime_id} healthcheck to use localhost or loopback, got {healthcheck}"
                    )))
                }
            }
        }
    }

    fn compare_runs(&self, reference: RuntimeRun, candidate: RuntimeRun) -> Report {
        let suite_by_case = suite_map(&reference.cases, &candidate.cases);
        let reference_by_id: BTreeMap<CaseId, Observation> = reference
            .observations
            .into_iter()
            .map(|observation| (observation.case_id.clone(), observation))
            .collect();
        let candidate_by_id: BTreeMap<CaseId, Observation> = candidate
            .observations
            .into_iter()
            .map(|observation| (observation.case_id.clone(), observation))
            .collect();

        let mut all_ids: BTreeSet<CaseId> = reference_by_id.keys().cloned().collect();
        all_ids.extend(candidate_by_id.keys().cloned());

        let mut divergences = Vec::new();
        divergences.extend(reference.divergences);
        divergences.extend(candidate.divergences);
        let mut normalizers_applied = Vec::new();
        let policy_engine = self.policy_engine();
        let normalize_ctx = NormalizeContext {
            project_root: Some(self.options.root.display().to_string()),
        };

        for case_id in &all_ids {
            match (reference_by_id.get(case_id), candidate_by_id.get(case_id)) {
                (Some(reference), Some(candidate)) => {
                    let reference_norm = policy_engine
                        .normalize(reference.clone(), &normalize_ctx)
                        .unwrap_or_else(|_| rewrit_core::NormalizationResult {
                            observation: reference.clone(),
                            applied: Vec::new(),
                        });
                    let candidate_norm = policy_engine
                        .normalize(candidate.clone(), &normalize_ctx)
                        .unwrap_or_else(|_| rewrit_core::NormalizationResult {
                            observation: candidate.clone(),
                            applied: Vec::new(),
                        });
                    normalizers_applied.extend(reference_norm.applied);
                    normalizers_applied.extend(candidate_norm.applied);
                    let applied_names = normalizers_applied
                        .iter()
                        .filter(|applied| applied.case_id == *case_id)
                        .map(|applied| applied.name.clone())
                        .collect();
                    let comparison = policy_engine.compare(
                        &reference_norm.observation,
                        &candidate_norm.observation,
                        CompareContext {
                            policy: policy_engine.policy.clone(),
                            suite: suite_by_case.get(case_id).cloned(),
                            source_location: None,
                            target_location: None,
                            normalizers_applied: applied_names,
                        },
                    );
                    divergences.extend(comparison.divergences);
                }
                (Some(_), None) => divergences.push(case_divergence(
                    case_id.clone(),
                    DivergenceKind::MissingCandidateCase,
                    "Candidate did not emit an observation for this required case.",
                )),
                (None, Some(_)) => {
                    let mut divergence = case_divergence(
                        case_id.clone(),
                        DivergenceKind::OrphanCandidateCase,
                        "Candidate emitted an observation with no reference case.",
                    );
                    if !policy_engine.policy.fail_on_orphan_candidate {
                        divergence.severity = Severity::Warning;
                    }
                    divergences.push(divergence);
                }
                (None, None) => {}
            }
        }
        attach_suites_to_divergences(&mut divergences, &suite_by_case);

        let mut report =
            self.report_from_divergences("run", reference.cases, Vec::new(), divergences);
        report.suites = suite_summaries(&suite_by_case, &all_ids, &report.divergences);
        report.summary.cases_discovered = report.summary.cases_discovered.max(all_ids.len());
        report.summary.cases_compared = all_ids.len();
        let blocking_case_ids = report
            .divergences
            .iter()
            .filter(|divergence| matches!(divergence.severity, Severity::Blocking))
            .map(|divergence| divergence.case_id.clone())
            .collect::<BTreeSet<_>>();
        report.summary.equivalent = all_ids.len().saturating_sub(blocking_case_ids.len());
        report.summary.parity_ratio = if report.summary.cases_compared == 0 {
            0.0
        } else {
            report.summary.equivalent as f64 / report.summary.cases_compared as f64
        };
        report.normalizers_applied = normalizers_applied;
        report.summary.exit_code = exit_code_for_report(&report);
        report
    }

    fn audit_runs(&self, reference: &RuntimeRun, candidate: &RuntimeRun) -> Vec<Divergence> {
        let reference_ids: BTreeSet<_> = reference
            .cases
            .iter()
            .map(|case| case.id.clone())
            .chain(reference.observations.iter().map(|obs| obs.case_id.clone()))
            .collect();
        let candidate_ids: BTreeSet<_> = candidate
            .cases
            .iter()
            .map(|case| case.id.clone())
            .chain(candidate.observations.iter().map(|obs| obs.case_id.clone()))
            .collect();

        let mut divergences = Vec::new();
        for case_id in reference_ids.difference(&candidate_ids) {
            divergences.push(case_divergence(
                case_id.clone(),
                DivergenceKind::MissingCandidateCase,
                "Candidate case is missing.",
            ));
        }
        for case_id in candidate_ids.difference(&reference_ids) {
            let mut divergence = case_divergence(
                case_id.clone(),
                DivergenceKind::OrphanCandidateCase,
                "Candidate case has no reference binding.",
            );
            if !self.policy_engine().policy.fail_on_orphan_candidate {
                divergence.severity = Severity::Warning;
            }
            divergences.push(divergence);
        }
        divergences
    }

    fn report_from_divergences(
        &self,
        command: &str,
        cases: Vec<Case>,
        observations: Vec<Observation>,
        divergences: Vec<Divergence>,
    ) -> Report {
        let blocking = divergences
            .iter()
            .filter(|divergence| matches!(divergence.severity, Severity::Blocking))
            .count();
        let warnings = divergences
            .iter()
            .filter(|divergence| matches!(divergence.severity, Severity::Warning))
            .count();
        let waived = divergences
            .iter()
            .filter(|divergence| matches!(divergence.severity, Severity::Allowed))
            .count();
        let discovered = cases.len().max(observations.len());
        let equivalent = discovered.saturating_sub(blocking);
        let mut report = Report {
            schema_version: rewrit_protocol::REPORT_SCHEMA_VERSION.to_string(),
            run_id: Uuid::now_v7().to_string(),
            project: self.manifest.project.name.clone(),
            reference: self.manifest.project.reference.to_string(),
            candidate: self.manifest.project.candidate.to_string(),
            summary: ReportSummary {
                cases_discovered: discovered,
                cases_compared: observations.len(),
                equivalent,
                waived,
                blocking,
                warnings,
                parity_ratio: if discovered == 0 {
                    0.0
                } else {
                    equivalent as f64 / discovered as f64
                },
                exit_code: 0,
            },
            suites: Vec::new(),
            divergences,
            normalizers_applied: Vec::new(),
            policy_trace: Vec::new(),
            metadata: BTreeMap::from([("command".to_string(), command.to_string())]),
        };
        self.attach_minimal_reproductions(command, &mut report.divergences);
        report.summary.exit_code = exit_code_for_report(&report);
        report
    }

    fn attach_minimal_reproductions(&self, command: &str, divergences: &mut [Divergence]) {
        for divergence in divergences {
            if divergence.minimal_reproduction.is_none() {
                divergence.minimal_reproduction =
                    Some(self.minimal_reproduction(command, &divergence.case_id));
            }
        }
    }

    fn minimal_reproduction(&self, command: &str, case_id: &CaseId) -> MinimalReproduction {
        let manifest_path = self.options.manifest_path.display().to_string();
        let args = match command {
            "audit" => vec!["audit".to_string(), "--manifest".to_string(), manifest_path],
            "doctor" => vec![
                "doctor".to_string(),
                "--manifest".to_string(),
                manifest_path,
            ],
            "capture" => vec![
                "capture".to_string(),
                "--manifest".to_string(),
                manifest_path,
                "--runtime".to_string(),
                self.manifest.project.reference.to_string(),
            ],
            "verify" => vec![
                "verify".to_string(),
                "--manifest".to_string(),
                manifest_path,
                "--runtime".to_string(),
                self.manifest.project.candidate.to_string(),
            ],
            _ => vec![
                "explain".to_string(),
                "--manifest".to_string(),
                manifest_path,
                case_id.to_string(),
            ],
        };

        MinimalReproduction {
            command: "rewrit".to_string(),
            args,
            cwd: Some(self.options.root.display().to_string()),
            env: BTreeMap::new(),
        }
    }

    fn write_configured_reports(&self, report: &Report) -> Result<(), EngineError> {
        let reports = if self.manifest.reports.is_empty() {
            vec![ReportConfig {
                kind: "json".to_string(),
                path: Some(".rewrit/reports/latest.json".to_string()),
            }]
        } else {
            self.manifest.reports.clone()
        };

        let _reports_lock = self.store.acquire_lock("reports")?;
        for config in reports {
            if config.kind == "terminal" && config.path.is_none() {
                continue;
            }
            let path = config.path.unwrap_or_else(|| {
                self.store
                    .reports_dir
                    .join(format!("latest.{}", extension_for_report(&config.kind)))
                    .display()
                    .to_string()
            });
            rewrit_report::write(&config.kind, self.options.root.join(path), report)?;
        }
        Ok(())
    }

    fn policy_engine(&self) -> PolicyEngine {
        let mut normalizers: Vec<Box<dyn Normalizer>> = Vec::new();
        for config in &self.manifest.normalizers {
            match config.kind.as_str() {
                "path" => normalizers.push(Box::new(PathNormalizer {
                    replacement: config
                        .replace_project_root
                        .clone()
                        .or(config.replacement.clone())
                        .unwrap_or_else(|| "<PROJECT_ROOT>".to_string()),
                })),
                "uuid" => normalizers.push(Box::new(
                    RegexNormalizer::uuid().with_paths(config.paths.clone()),
                )),
                "timestamp" => normalizers.push(Box::new(
                    timestamp_normalizer().with_paths(config.paths.clone()),
                )),
                "regex" => {
                    if let (Some(pattern), Some(replacement)) =
                        (&config.pattern, &config.replacement)
                    {
                        if let Ok(normalizer) =
                            RegexNormalizer::new("regex", pattern, replacement.clone())
                        {
                            normalizers.push(Box::new(normalizer.with_paths(config.paths.clone())));
                        }
                    }
                }
                "http_headers" => normalizers.push(Box::new(HttpHeaderNormalizer)),
                _ => {}
            }
        }

        PolicyEngine {
            normalizers: NormalizationPipeline::new(normalizers),
            comparator: Box::new(StrictComparator) as Box<dyn Comparator>,
            waivers: WaiverSet::new(
                self.manifest
                    .waivers
                    .iter()
                    .map(|waiver| Waiver {
                        case: waiver.case.clone(),
                        kind: waiver.kind.clone(),
                        reason: waiver.reason.clone(),
                        owner: waiver.owner.clone(),
                        expires: waiver.expires.clone(),
                        issue: waiver.issue.clone(),
                    })
                    .collect(),
            ),
            policy: self.policy_from_manifest(),
        }
    }

    fn policy_from_manifest(&self) -> Policy {
        let mut policy = Policy::default();
        if let Some((name, config)) = self.manifest.policies.iter().next() {
            policy.name = name.clone();
            apply_policy_config(&mut policy, config);
        }
        policy.db_maps = self.manifest.effects.db.maps.clone();
        policy
    }
}

fn suite_map(reference_cases: &[Case], candidate_cases: &[Case]) -> BTreeMap<CaseId, String> {
    let mut suites = BTreeMap::new();
    for case in candidate_cases {
        suites.insert(case.id.clone(), case.suite_id.to_string());
    }
    for case in reference_cases {
        suites.insert(case.id.clone(), case.suite_id.to_string());
    }
    suites
}

fn audit_case_ids(reference: &RuntimeRun, candidate: &RuntimeRun) -> BTreeSet<CaseId> {
    reference
        .cases
        .iter()
        .map(|case| case.id.clone())
        .chain(reference.observations.iter().map(|obs| obs.case_id.clone()))
        .chain(candidate.cases.iter().map(|case| case.id.clone()))
        .chain(candidate.observations.iter().map(|obs| obs.case_id.clone()))
        .collect()
}

fn attach_suites_to_divergences(
    divergences: &mut [Divergence],
    suite_by_case: &BTreeMap<CaseId, String>,
) {
    for divergence in divergences {
        if divergence.suite.is_none() {
            divergence.suite = suite_by_case.get(&divergence.case_id).cloned();
        }
    }
}

fn suite_summaries(
    suite_by_case: &BTreeMap<CaseId, String>,
    all_case_ids: &BTreeSet<CaseId>,
    divergences: &[Divergence],
) -> Vec<SuiteSummary> {
    let mut cases_by_suite: BTreeMap<String, BTreeSet<CaseId>> = BTreeMap::new();
    for case_id in all_case_ids {
        if let Some(suite_id) = suite_by_case.get(case_id) {
            cases_by_suite
                .entry(suite_id.clone())
                .or_default()
                .insert(case_id.clone());
        }
    }

    let mut blocking_by_suite: BTreeMap<String, BTreeSet<CaseId>> = BTreeMap::new();
    for divergence in divergences
        .iter()
        .filter(|divergence| matches!(divergence.severity, Severity::Blocking))
    {
        let suite_id = divergence
            .suite
            .clone()
            .or_else(|| suite_by_case.get(&divergence.case_id).cloned());
        if let Some(suite_id) = suite_id {
            blocking_by_suite
                .entry(suite_id)
                .or_default()
                .insert(divergence.case_id.clone());
        }
    }

    let mut summaries = cases_by_suite
        .into_iter()
        .map(|(suite_id, case_ids)| {
            let cases_compared = case_ids.len();
            let blocking = blocking_by_suite
                .get(&suite_id)
                .map_or(0, BTreeSet::len)
                .min(cases_compared);
            let equivalent = cases_compared.saturating_sub(blocking);
            SuiteSummary {
                suite_id,
                cases_compared,
                equivalent,
                blocking,
                parity_ratio: if cases_compared == 0 {
                    0.0
                } else {
                    equivalent as f64 / cases_compared as f64
                },
            }
        })
        .collect::<Vec<_>>();

    summaries.sort_by(|left, right| {
        left.parity_ratio
            .partial_cmp(&right.parity_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.suite_id.cmp(&right.suite_id))
    });
    summaries
}

fn validate_manifest(manifest: &Manifest) -> Result<(), EngineError> {
    if manifest.project.name.trim().is_empty() {
        return Err(EngineError::InvalidManifest(
            "project.name is required".to_string(),
        ));
    }
    if manifest.runtimes.is_empty() {
        return Err(EngineError::InvalidManifest(
            "at least one runtime is required".to_string(),
        ));
    }
    if !manifest.runtimes.contains_key(&manifest.project.reference) {
        return Err(EngineError::InvalidManifest(format!(
            "reference runtime {} is not defined",
            manifest.project.reference
        )));
    }
    if !manifest.runtimes.contains_key(&manifest.project.candidate) {
        return Err(EngineError::InvalidManifest(format!(
            "candidate runtime {} is not defined",
            manifest.project.candidate
        )));
    }
    for waiver in &manifest.waivers {
        if waiver.reason.trim().is_empty() {
            return Err(EngineError::InvalidManifest(format!(
                "waiver for {} is missing reason",
                waiver.case
            )));
        }
        if waiver.owner.trim().is_empty() {
            return Err(EngineError::InvalidManifest(format!(
                "waiver for {} is missing owner",
                waiver.case
            )));
        }
        if waiver.expires.trim().is_empty() {
            return Err(EngineError::InvalidManifest(format!(
                "waiver for {} is missing expires",
                waiver.case
            )));
        }
    }
    if manifest.security.sandbox.enabled
        && manifest
            .security
            .sandbox
            .image
            .as_deref()
            .map_or(true, str::is_empty)
    {
        return Err(EngineError::InvalidManifest(
            "security.sandbox.image is required when sandbox is enabled".to_string(),
        ));
    }
    Ok(())
}

fn collect_contract_files(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<(), EngineError> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let skip = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| matches!(name, ".git" | ".rewrit" | "target"))
                .unwrap_or(false);
            if !skip {
                collect_contract_files(&path, paths)?;
            }
            continue;
        }

        let is_contract = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| matches!(extension, "json" | "yaml" | "yml"))
            .unwrap_or(false);
        if is_contract {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(())
}

fn absolute_path(path: PathBuf) -> Result<PathBuf, EngineError> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn case_from_contract(loaded: &LoadedContract) -> Case {
    Case {
        id: loaded.contract.id.clone(),
        suite_id: SuiteId::new("contracts"),
        title: loaded.contract.id.to_string(),
        source_location: Some(SourceLocation {
            path: loaded.path.display().to_string(),
            line: None,
            column: None,
        }),
        tags: vec![loaded.contract.kind.clone()],
        contract_ref: Some(ContractRef(loaded.path.display().to_string())),
        required: true,
    }
}

fn apply_contracts_to_command_run(
    contracts: &[LoadedContract],
    runtime_id: &RuntimeId,
    run: &mut RuntimeRun,
    missing_kind: DivergenceKind,
) {
    let mut known_cases = run
        .cases
        .iter()
        .map(|case| case.id.clone())
        .collect::<BTreeSet<_>>();
    for loaded in contracts {
        if known_cases.insert(loaded.contract.id.clone()) {
            run.cases.push(case_from_contract(loaded));
        }
    }

    let observations = run
        .observations
        .iter()
        .map(|observation| (observation.case_id.clone(), observation))
        .collect::<BTreeMap<_, _>>();
    for loaded in contracts {
        match observations.get(&loaded.contract.id) {
            Some(observation) => run
                .divergences
                .extend(validate_command_contract_observation(
                    &loaded.contract,
                    observation,
                    runtime_id,
                )),
            None => run.divergences.push(contract_observation_missing(
                &loaded.contract,
                runtime_id,
                missing_kind.clone(),
            )),
        }
    }
}

fn http_adapter_error_observation(
    runtime_id: &RuntimeId,
    case_id: &CaseId,
    error: HttpAdapterError,
) -> Observation {
    Observation {
        case_id: case_id.clone(),
        runtime_id: runtime_id.clone(),
        status: CaseStatus::AdapterError,
        value: None,
        error: None,
        stdout: CapturedText::default(),
        stderr: CapturedText::new(error.to_string()),
        exit_code: None,
        duration_ms: 0,
        effects: Vec::new(),
        artifacts: Vec::new(),
        metadata: BTreeMap::new(),
    }
}

fn is_loopback_url(url: &str) -> bool {
    let Some(after_scheme) = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
    else {
        return false;
    };
    let authority = after_scheme.split('/').next().unwrap_or_default();
    let host = if let Some(rest) = authority.strip_prefix('[') {
        rest.split(']').next().unwrap_or_default()
    } else {
        authority.split(':').next().unwrap_or_default()
    };
    host == "localhost" || host == "::1" || host.starts_with("127.")
}

fn validate_http_contract_observation(
    contract: &Contract,
    observation: &Observation,
) -> Vec<Divergence> {
    let mut divergences = Vec::new();
    let Some(value) = observation.value.as_ref() else {
        return vec![contract_divergence(
            contract.id.clone(),
            DivergenceKind::SchemaMismatch,
            "$.value",
            "Observation has no HTTP value to validate against the contract.",
            contract.expect.json_schema.as_ref(),
            Option::<&serde_json::Value>::None,
        )];
    };

    if let Some(expected_status) = contract.expect.status {
        match http_status(value) {
            Some(actual_status) if actual_status == expected_status => {}
            actual_status => {
                let expected = serde_json::json!(expected_status);
                let actual = actual_status.map(|status| serde_json::json!(status));
                divergences.push(contract_divergence(
                    contract.id.clone(),
                    DivergenceKind::OutputMismatch,
                    "$.value.status",
                    format!(
                        "HTTP status does not match contract expectation: expected {expected_status}, got {}.",
                        actual_status
                            .map(|status| status.to_string())
                            .unwrap_or_else(|| "<missing>".to_string())
                    ),
                    Some(&expected),
                    actual.as_ref(),
                ));
            }
        }
    }

    if let Some(schema) = &contract.expect.json_schema {
        match http_body_json(value) {
            Some(body) => divergences.extend(validate_json_schema(
                &contract.id,
                schema,
                body,
                "$.value.body",
            )),
            None => divergences.push(contract_divergence(
                contract.id.clone(),
                DivergenceKind::SchemaMismatch,
                "$.value.body",
                "HTTP body is not JSON and cannot be validated against json_schema.",
                Some(schema),
                Option::<&serde_json::Value>::None,
            )),
        }
    }

    divergences
}

fn validate_command_contract_observation(
    contract: &Contract,
    observation: &Observation,
    runtime_id: &RuntimeId,
) -> Vec<Divergence> {
    if contract.kind == "http_case" {
        return validate_http_contract_observation(contract, observation);
    }

    let mut divergences = Vec::new();

    if let Some(expected) = &contract.expect.json {
        match observation.value.as_ref().and_then(canonical_to_json) {
            Some(actual) if actual == *expected => {}
            Some(actual) => divergences.push(contract_divergence(
                contract.id.clone(),
                DivergenceKind::OutputMismatch,
                "$.value",
                format!("Runtime {runtime_id} value does not match contract expectation."),
                Some(expected),
                Some(&actual),
            )),
            None => divergences.push(contract_divergence(
                contract.id.clone(),
                DivergenceKind::SchemaMismatch,
                "$.value",
                format!("Runtime {runtime_id} observation has no JSON-compatible value."),
                Some(expected),
                Option::<&serde_json::Value>::None,
            )),
        }
    }

    if let Some(schema) = &contract.expect.json_schema {
        match observation.value.as_ref().and_then(canonical_to_json) {
            Some(actual) => {
                divergences.extend(validate_json_schema(
                    &contract.id,
                    schema,
                    &actual,
                    "$.value",
                ));
            }
            None => divergences.push(contract_divergence(
                contract.id.clone(),
                DivergenceKind::SchemaMismatch,
                "$.value",
                format!(
                    "Runtime {runtime_id} observation has no JSON-compatible value for json_schema."
                ),
                Some(schema),
                Option::<&serde_json::Value>::None,
            )),
        }
    }

    if !contract.expect.effects.is_empty() {
        let expected = serde_json::Value::Array(contract.expect.effects.clone());
        let actual =
            serde_json::to_value(&observation.effects).unwrap_or_else(|_| serde_json::json!([]));
        if actual != expected {
            divergences.push(contract_divergence(
                contract.id.clone(),
                DivergenceKind::SideEffectMismatch,
                "$.effects",
                format!("Runtime {runtime_id} side effects do not match contract expectation."),
                Some(&expected),
                Some(&actual),
            ));
        }
    }

    divergences
}

fn http_status(value: &rewrit_model::CanonicalValue) -> Option<u16> {
    let rewrit_model::CanonicalValue::Object { fields } = value else {
        return None;
    };
    let rewrit_model::CanonicalValue::Integer { value } = fields.get("status")? else {
        return None;
    };
    value.parse().ok()
}

fn http_body_json(value: &rewrit_model::CanonicalValue) -> Option<&serde_json::Value> {
    let rewrit_model::CanonicalValue::Object { fields } = value else {
        return None;
    };
    match fields.get("body")? {
        rewrit_model::CanonicalValue::Json { value } => Some(value),
        _ => None,
    }
}

fn canonical_to_json(value: &CanonicalValue) -> Option<serde_json::Value> {
    match value {
        CanonicalValue::Null => Some(serde_json::Value::Null),
        CanonicalValue::Absent => None,
        CanonicalValue::Bool { value } => Some(serde_json::json!(value)),
        CanonicalValue::Integer { value } => value
            .parse::<i64>()
            .map(|integer| serde_json::json!(integer))
            .or_else(|_| {
                value
                    .parse::<u64>()
                    .map(|integer| serde_json::json!(integer))
            })
            .ok()
            .or_else(|| Some(serde_json::json!(value))),
        CanonicalValue::Decimal { value }
        | CanonicalValue::Float { value }
        | CanonicalValue::String { value } => Some(serde_json::json!(value)),
        CanonicalValue::Bytes { base64, .. } => Some(serde_json::json!(base64)),
        CanonicalValue::Array { items } => items
            .iter()
            .map(canonical_to_json)
            .collect::<Option<Vec<_>>>()
            .map(serde_json::Value::Array),
        CanonicalValue::Object { fields } => fields
            .iter()
            .map(|(key, value)| canonical_to_json(value).map(|value| (key.clone(), value)))
            .collect::<Option<serde_json::Map<_, _>>>()
            .map(serde_json::Value::Object),
        CanonicalValue::DateTime { rfc3339 } => Some(serde_json::json!(rfc3339)),
        CanonicalValue::Json { value } => Some(value.clone()),
    }
}

fn validate_json_schema(
    case_id: &CaseId,
    schema: &serde_json::Value,
    value: &serde_json::Value,
    path: &str,
) -> Vec<Divergence> {
    let mut divergences = Vec::new();
    if let Some(expected_type) = schema.get("type").and_then(serde_json::Value::as_str) {
        if !json_type_matches(expected_type, value) {
            divergences.push(contract_divergence(
                case_id.clone(),
                DivergenceKind::SchemaMismatch,
                path,
                format!(
                    "JSON schema type mismatch: expected {expected_type}, got {}.",
                    json_kind(value)
                ),
                Some(schema),
                Some(value),
            ));
            return divergences;
        }
    }

    if let Some(expected_const) = schema.get("const") {
        if value != expected_const {
            divergences.push(contract_divergence(
                case_id.clone(),
                DivergenceKind::SchemaMismatch,
                path,
                "JSON value does not match schema const.",
                Some(expected_const),
                Some(value),
            ));
        }
    }

    if let Some(pattern) = schema.get("pattern").and_then(serde_json::Value::as_str) {
        match value.as_str() {
            Some(text) if regex::Regex::new(pattern).is_ok_and(|regex| regex.is_match(text)) => {}
            _ => divergences.push(contract_divergence(
                case_id.clone(),
                DivergenceKind::SchemaMismatch,
                path,
                format!("JSON string does not match schema pattern {pattern}."),
                Some(schema),
                Some(value),
            )),
        }
    }

    if let serde_json::Value::Object(object) = value {
        if let Some(required) = schema.get("required").and_then(serde_json::Value::as_array) {
            for required_field in required.iter().filter_map(serde_json::Value::as_str) {
                if !object.contains_key(required_field) {
                    let expected = serde_json::json!(required_field);
                    divergences.push(contract_divergence(
                        case_id.clone(),
                        DivergenceKind::SchemaMismatch,
                        format!("{path}.{required_field}"),
                        "JSON object is missing a schema-required field.",
                        Some(&expected),
                        Option::<&serde_json::Value>::None,
                    ));
                }
            }
        }

        if let Some(properties) = schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
        {
            for (property, property_schema) in properties {
                if let Some(property_value) = object.get(property) {
                    divergences.extend(validate_json_schema(
                        case_id,
                        property_schema,
                        property_value,
                        &format!("{path}.{property}"),
                    ));
                }
            }
        }
    }

    divergences
}

fn json_type_matches(expected_type: &str, value: &serde_json::Value) -> bool {
    match expected_type {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => true,
    }
}

fn json_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(number) if number.is_i64() || number.is_u64() => "integer",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn contract_observation_missing(
    contract: &Contract,
    runtime_id: &RuntimeId,
    kind: DivergenceKind,
) -> Divergence {
    let runtime = serde_json::json!(runtime_id.to_string());
    contract_divergence(
        contract.id.clone(),
        kind,
        "$.observation",
        format!("Runtime {runtime_id} did not emit an observation for this contract."),
        Some(&runtime),
        Option::<&serde_json::Value>::None,
    )
}

fn contract_divergence<T, U>(
    case_id: CaseId,
    kind: DivergenceKind,
    path: impl Into<String>,
    message: impl Into<String>,
    reference: Option<&T>,
    candidate: Option<&U>,
) -> Divergence
where
    T: serde::Serialize + ?Sized,
    U: serde::Serialize + ?Sized,
{
    Divergence {
        machine_code: format!("{kind:?}").to_ascii_lowercase(),
        kind,
        severity: Severity::Blocking,
        case_id,
        suite: Some("contracts".to_string()),
        path: Some(path.into()),
        reference: reference.and_then(|value| serde_json::to_value(value).ok()),
        candidate: candidate.and_then(|value| serde_json::to_value(value).ok()),
        message: message.into(),
        source_location: None,
        target_location: None,
        policy: Some("contract".to_string()),
        normalizers_applied: Vec::new(),
        hint: Some("Align the runtime response with the declared contract.".to_string()),
        minimal_reproduction: None,
    }
}

fn apply_policy_config(policy: &mut Policy, config: &PolicyConfig) {
    if let Some(value) = config.compare_stdout {
        policy.compare_stdout = value;
    }
    if let Some(value) = config.compare_stderr {
        policy.compare_stderr = value;
    }
    if let Some(value) = config.compare_exit_code {
        policy.compare_exit_code = value;
    }
    if let Some(value) = config.compare_duration {
        policy.compare_duration = value;
    }
    if let Some(value) = config.allow_null_absent_equivalence {
        policy.allow_null_absent_equivalence = value;
    }
    if let Some(value) = config.allow_integer_float_equivalence {
        policy.allow_integer_float_equivalence = value;
    }
    if let Some(value) = config.allow_header_case_difference {
        policy.allow_header_case_difference = value;
    }
    if let Some(value) = config.allow_object_key_order_difference {
        policy.allow_object_key_order_difference = value;
    }
    if let Some(value) = config.fail_on_orphan_candidate {
        policy.fail_on_orphan_candidate = value;
    }
    if let Some(value) = config.decimal_as_string {
        policy.decimal_as_string = value;
    }
    if let Some(value) = config.ignore_stack_trace {
        policy.ignore_stack_trace = value;
    }
    if let Some(headers) = &config.headers {
        policy.ignored_headers = headers
            .ignore
            .iter()
            .map(|header| header.to_ascii_lowercase())
            .collect();
    }
    if let Some(json) = &config.json {
        policy.ignore_paths = json.ignore_paths.clone();
        policy.unordered_paths = json.unordered_paths.clone();
    }
}

fn case_divergence(
    case_id: CaseId,
    kind: DivergenceKind,
    message: impl Into<String>,
) -> Divergence {
    Divergence {
        machine_code: format!("{kind:?}").to_ascii_lowercase(),
        kind,
        severity: Severity::Blocking,
        case_id,
        suite: None,
        path: None,
        reference: None,
        candidate: None,
        message: message.into(),
        source_location: None,
        target_location: None,
        policy: Some("audit".to_string()),
        normalizers_applied: Vec::new(),
        hint: None,
        minimal_reproduction: None,
    }
}

fn runtime_divergence(
    runtime_id: &RuntimeId,
    kind: DivergenceKind,
    message: impl Into<String>,
) -> Divergence {
    case_divergence(CaseId::new(format!("{runtime_id}.doctor")), kind, message)
}

#[must_use]
pub fn exit_code_for_report(report: &Report) -> i32 {
    if report.summary.blocking > 0 {
        1
    } else if report.summary.cases_discovered == 0
        && report.metadata.get("command").map(String::as_str) != Some("doctor")
    {
        8
    } else {
        0
    }
}

fn adapter_command_name(command: AdapterCommand) -> &'static str {
    match command {
        AdapterCommand::Doctor => "doctor",
        AdapterCommand::Discover => "discover",
        AdapterCommand::Run => "run",
    }
}

fn is_command_protocol_adapter(adapter: &str) -> bool {
    adapter == "command"
        || adapter.starts_with("command:")
        || adapter == "rust:cargo_test"
        || adapter.starts_with("rust:cargo_test:")
}

fn protocol_input_name(input: AdapterProtocolInput) -> &'static str {
    match input {
        AdapterProtocolInput::None => "none",
        AdapterProtocolInput::File => "file",
    }
}

fn protocol_output_name(output: AdapterProtocolOutput) -> &'static str {
    match output {
        AdapterProtocolOutput::Stdout => "stdout",
        AdapterProtocolOutput::File => "file",
    }
}

fn extension_for_report(kind: &str) -> &'static str {
    match kind {
        "json" => "json",
        "ndjson" => "ndjson",
        "junit" => "xml",
        "sarif" => "sarif",
        "html" => "html",
        "markdown" => "md",
        _ => "txt",
    }
}
