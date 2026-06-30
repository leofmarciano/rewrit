use crate::discovery::manifest::{
    Manifest as ManifestConfig, PolicyConfig, ReportConfig, RuntimeConfig,
};
use crate::runner::process::{ProcessError, ProcessRunner};
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
    CapturedText, Case, CaseId, CaseStatus, Contract, ContractRef, Divergence, DivergenceKind,
    Observation, Report, ReportSummary, RuntimeId, Severity, SourceLocation, SuiteId,
};
use rewrit_protocol::{decode_events, AdapterEvent};
use std::collections::{BTreeMap, BTreeSet};
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
    #[error("adapter protocol error for runtime {runtime_id}: {source}")]
    Protocol {
        runtime_id: RuntimeId,
        #[source]
        source: rewrit_protocol::ProtocolError,
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
            Self::Protocol { .. } | Self::HttpAdapter { .. } => 4,
            Self::RuntimeNotFound(_)
            | Self::RuntimeFailed { .. }
            | Self::StartServer { .. }
            | Self::Baseline(_) => 5,
            Self::Report(_) => 7,
            Self::Io(_) => 70,
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
        let mut divergences = Vec::new();
        for runtime_id in self.manifest.runtimes.keys() {
            if let Err(error) = self.run_runtime(runtime_id).await {
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
        let runtime_ids: Vec<RuntimeId> = match runtime_id {
            Some(runtime_id) => vec![runtime_id.clone()],
            None => self.manifest.runtimes.keys().cloned().collect(),
        };

        let mut cases = Vec::new();
        for runtime_id in runtime_ids {
            cases.extend(self.run_runtime(&runtime_id).await?.cases);
        }
        Ok(cases)
    }

    pub async fn run(&self, _mode: RunMode) -> Result<Report, EngineError> {
        let reference = self.run_runtime(&self.manifest.project.reference).await?;
        let candidate = self.run_runtime(&self.manifest.project.candidate).await?;
        let report = self.compare_runs(reference, candidate);
        self.write_configured_reports(&report)?;
        Ok(report)
    }

    pub async fn capture(&self, runtime_id: &RuntimeId) -> Result<Report, EngineError> {
        let run = self.run_runtime(runtime_id).await?;
        baseline::write_current(&self.store, runtime_id, &run.observations)?;
        let report =
            self.report_from_divergences("capture", run.cases, run.observations, Vec::new());
        self.write_configured_reports(&report)?;
        Ok(report)
    }

    pub async fn verify(&self, runtime_id: &RuntimeId) -> Result<Report, EngineError> {
        let baseline_runtime = &self.manifest.project.reference;
        let baseline_observations = baseline::read_current(&self.store, baseline_runtime)?;
        let reference = RuntimeRun {
            cases: Vec::new(),
            observations: baseline_observations,
        };
        let candidate = self.run_runtime(runtime_id).await?;
        let report = self.compare_runs(reference, candidate);
        self.write_configured_reports(&report)?;
        Ok(report)
    }

    pub async fn verify_contracts(&self, contract_paths: &[String]) -> Result<Report, EngineError> {
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
        let reference = if reference_runtime.adapter.starts_with("http") {
            self.run_http_runtime(
                &self.manifest.project.reference,
                reference_runtime,
                Some(&contracts),
            )
            .await?
        } else {
            self.run_runtime(&self.manifest.project.reference).await?
        };
        let candidate = if candidate_runtime.adapter.starts_with("http") {
            self.run_http_runtime(
                &self.manifest.project.candidate,
                candidate_runtime,
                Some(&contracts),
            )
            .await?
        } else {
            self.run_runtime(&self.manifest.project.candidate).await?
        };
        let report = self.compare_runs(reference, candidate);
        self.write_configured_reports(&report)?;
        Ok(report)
    }

    pub async fn audit(&self) -> Result<Report, EngineError> {
        let reference = self.run_runtime(&self.manifest.project.reference).await?;
        let candidate = self.run_runtime(&self.manifest.project.candidate).await?;
        let divergences = self.audit_runs(&reference, &candidate);
        let report = self.report_from_divergences(
            "audit",
            reference.cases,
            candidate.observations,
            divergences,
        );
        self.write_configured_reports(&report)?;
        Ok(report)
    }

    pub async fn explain(&self, case_id: &CaseId) -> Result<ExplainResult, EngineError> {
        let report = self.run(RunMode::Mirror).await?;
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

    async fn run_runtime(&self, runtime_id: &RuntimeId) -> Result<RuntimeRun, EngineError> {
        let runtime = self
            .manifest
            .runtimes
            .get(runtime_id)
            .ok_or_else(|| EngineError::RuntimeNotFound(runtime_id.clone()))?;

        if runtime.adapter != "command" && !runtime.adapter.starts_with("command") {
            if runtime.adapter == "http" || runtime.adapter.starts_with("http:") {
                return self.run_http_runtime(runtime_id, runtime, None).await;
            }
            return Ok(RuntimeRun::default());
        }

        self.run_command_runtime(runtime_id, runtime).await
    }

    async fn run_command_runtime(
        &self,
        runtime_id: &RuntimeId,
        runtime: &RuntimeConfig,
    ) -> Result<RuntimeRun, EngineError> {
        let process = self.runner.from_runtime(&self.options.root, runtime);
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
            });
        }

        let events = decode_events(&output.stdout).map_err(|source| EngineError::Protocol {
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
                Ok(observation) => observations.push(observation),
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
        command
            .args(args)
            .current_dir(cwd)
            .envs(&runtime.env)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(self.manifest.runner.kill_process_tree);

        command
            .spawn()
            .map(Some)
            .map_err(|source| EngineError::StartServer {
                runtime_id: runtime_id.clone(),
                source,
            })
    }

    fn compare_runs(&self, reference: RuntimeRun, candidate: RuntimeRun) -> Report {
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
        let mut equivalent = 0usize;
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
                            suite: None,
                            source_location: None,
                            target_location: None,
                            normalizers_applied: applied_names,
                        },
                    );
                    if comparison.equivalent {
                        equivalent += 1;
                    }
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

        let mut report =
            self.report_from_divergences("run", reference.cases, Vec::new(), divergences);
        report.summary.cases_compared = all_ids.len();
        report.summary.equivalent = equivalent;
        report.summary.parity_ratio = if report.summary.cases_compared == 0 {
            0.0
        } else {
            equivalent as f64 / report.summary.cases_compared as f64
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
        report.summary.exit_code = exit_code_for_report(&report);
        report
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
                "uuid" => normalizers.push(Box::new(RegexNormalizer::uuid())),
                "timestamp" => normalizers.push(Box::new(timestamp_normalizer())),
                "regex" => {
                    if let (Some(pattern), Some(replacement)) =
                        (&config.pattern, &config.replacement)
                    {
                        if let Ok(normalizer) =
                            RegexNormalizer::new("regex", pattern, replacement.clone())
                        {
                            normalizers.push(Box::new(normalizer));
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
        policy
    }
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
