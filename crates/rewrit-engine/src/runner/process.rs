use crate::discovery::manifest::{NetworkMode, RunnerConfig, RuntimeConfig, SecurityConfig};
use crate::runner::env::{truncate, Redactor};
use crate::runner::sandbox::{SandboxConfig, SandboxNetwork};
use crate::runner::timeout::millis;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct RuntimeProcess {
    pub command: Vec<String>,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    pub temp_dir: Option<PathBuf>,
    pub timeout_ms: u64,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
    pub sandbox: SandboxConfig,
}

#[derive(Debug, Clone)]
pub struct ProcessOutput {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stdout_truncated: bool,
    pub stderr: String,
    pub stderr_truncated: bool,
    pub timed_out: bool,
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("runtime command is empty")]
    EmptyCommand,
    #[error("failed to spawn runtime command: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("runtime command timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
}

#[derive(Debug, Clone)]
pub struct ProcessRunner {
    runner: RunnerConfig,
    redactor: Redactor,
    env_allowlist: Vec<String>,
    network_mode: NetworkMode,
    sandbox: SandboxConfig,
}

impl ProcessRunner {
    #[must_use]
    pub fn new(runner: RunnerConfig, security: &SecurityConfig) -> Self {
        Self {
            runner,
            redactor: Redactor::new(&security.redact_patterns),
            env_allowlist: security.env_allowlist.clone(),
            network_mode: security.network_mode,
            sandbox: security.sandbox.clone(),
        }
    }

    #[must_use]
    pub fn from_runtime(&self, root: &Path, runtime: &RuntimeConfig) -> RuntimeProcess {
        RuntimeProcess {
            command: runtime.command.clone(),
            cwd: runtime
                .cwd
                .as_ref()
                .map(|cwd| root.join(cwd))
                .unwrap_or_else(|| root.to_path_buf()),
            env: runtime.env.clone(),
            temp_dir: None,
            timeout_ms: runtime
                .timeout_ms
                .or(self.runner.default_timeout_ms)
                .unwrap_or(30_000),
            max_stdout_bytes: self.runner.max_stdout_bytes.unwrap_or(1_048_576),
            max_stderr_bytes: self.runner.max_stderr_bytes.unwrap_or(1_048_576),
            sandbox: self.sandbox.clone(),
        }
    }

    pub fn apply_environment(
        &self,
        command: &mut Command,
        runtime_env: &BTreeMap<String, String>,
        temp_dir: Option<&Path>,
    ) {
        if !self.env_allowlist.is_empty() {
            command.env_clear();
            for (key, value) in std::env::vars() {
                if env_allowed(&key, &self.env_allowlist) {
                    command.env(key, value);
                }
            }
        }
        command.envs(runtime_env);
        command.env("REWRIT_NETWORK_MODE", network_mode_name(self.network_mode));
        if let Some(temp_dir) = temp_dir {
            command
                .env("TMPDIR", temp_dir)
                .env("TMP", temp_dir)
                .env("TEMP", temp_dir);
        }
    }

    pub async fn run(&self, process: &RuntimeProcess) -> Result<ProcessOutput, ProcessError> {
        let Some((_program, _args)) = process.command.split_first() else {
            return Err(ProcessError::EmptyCommand);
        };
        let (program, args) = command_with_optional_sandbox(process, self.network_mode);

        let mut command = Command::new(program);
        command
            .args(args.iter())
            .current_dir(&process.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(self.runner.kill_process_tree);
        if let Some(temp_dir) = &process.temp_dir {
            std::fs::create_dir_all(temp_dir)?;
        }
        self.apply_environment(&mut command, &process.env, process.temp_dir.as_deref());

        let child = command.spawn()?;
        let output = match timeout(millis(process.timeout_ms), child.wait_with_output()).await {
            Ok(result) => result?,
            Err(_) => {
                return Ok(ProcessOutput {
                    status_code: None,
                    stdout: String::new(),
                    stdout_truncated: false,
                    stderr: format!("runtime timed out after {}ms", process.timeout_ms),
                    stderr_truncated: false,
                    timed_out: true,
                });
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let stdout = self.redactor.redact(&stdout);
        let stderr = self.redactor.redact(&stderr);
        let (stdout, stdout_truncated) = truncate(stdout, process.max_stdout_bytes);
        let (stderr, stderr_truncated) = truncate(stderr, process.max_stderr_bytes);

        Ok(ProcessOutput {
            status_code: output.status.code(),
            stdout,
            stdout_truncated,
            stderr,
            stderr_truncated,
            timed_out: false,
        })
    }
}

fn command_with_optional_sandbox(
    process: &RuntimeProcess,
    network_mode: NetworkMode,
) -> (String, Vec<String>) {
    let Some((program, args)) = process.command.split_first() else {
        return (String::new(), Vec::new());
    };
    if !process.sandbox.enabled {
        return (program.clone(), args.to_vec());
    }

    let sandbox = &process.sandbox;
    let image = sandbox.image.clone().unwrap_or_default();
    let mut wrapped = vec!["run".to_string(), "--rm".to_string(), "-i".to_string()];

    match sandbox.network {
        SandboxNetwork::Inherit => {}
        SandboxNetwork::Disabled => wrapped.push("--network=none".to_string()),
        SandboxNetwork::Host => wrapped.push("--network=host".to_string()),
    }

    wrapped.extend(sandbox.extra_args.clone());
    wrapped.extend([
        "-v".to_string(),
        format!("{}:{}", process.cwd.display(), process.cwd.display()),
        "-w".to_string(),
        process.cwd.display().to_string(),
    ]);
    if let Some(temp_dir) = &process.temp_dir {
        wrapped.extend([
            "-v".to_string(),
            format!("{}:{}", temp_dir.display(), temp_dir.display()),
            "-e".to_string(),
            format!("TMPDIR={}", temp_dir.display()),
            "-e".to_string(),
            format!("TMP={}", temp_dir.display()),
            "-e".to_string(),
            format!("TEMP={}", temp_dir.display()),
        ]);
    }

    wrapped.extend([
        "-e".to_string(),
        format!("REWRIT_NETWORK_MODE={}", network_mode_name(network_mode)),
    ]);
    for (key, value) in &process.env {
        wrapped.extend(["-e".to_string(), format!("{key}={value}")]);
    }

    wrapped.push(image);
    wrapped.push(program.clone());
    wrapped.extend(args.iter().cloned());

    (sandbox.engine.command().to_string(), wrapped)
}

fn env_allowed(key: &str, allowlist: &[String]) -> bool {
    allowlist.iter().any(|entry| {
        if let Some(prefix) = entry.strip_suffix('*') {
            key.starts_with(prefix)
        } else {
            key == entry
        }
    })
}

fn network_mode_name(mode: NetworkMode) -> &'static str {
    match mode {
        NetworkMode::Inherit => "inherit",
        NetworkMode::LoopbackOnly => "loopback_only",
        NetworkMode::Disabled => "disabled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::manifest::SecurityConfig;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[tokio::test]
    async fn env_allowlist_filters_inherited_env_but_keeps_runtime_env() {
        {
            let _guard = ENV_LOCK.lock().expect("env lock");
            std::env::set_var("REWRIT_ALLOWLISTED_TEST_ENV", "allowed");
            std::env::set_var("REWRIT_BLOCKED_TEST_ENV", "blocked");
        }

        let runner = ProcessRunner::new(
            RunnerConfig::default(),
            &SecurityConfig {
                env_allowlist: vec!["REWRIT_ALLOWLISTED_*".to_string()],
                ..SecurityConfig::default()
            },
        );
        let process = RuntimeProcess {
            command: vec!["/usr/bin/env".to_string()],
            cwd: std::env::current_dir().expect("cwd"),
            env: BTreeMap::from([("REWRIT_RUNTIME_TEST_ENV".to_string(), "runtime".to_string())]),
            temp_dir: None,
            timeout_ms: 30_000,
            max_stdout_bytes: 1_048_576,
            max_stderr_bytes: 1_048_576,
            sandbox: SandboxConfig::default(),
        };

        let output = runner.run(&process).await.expect("run");

        assert!(output
            .stdout
            .contains("REWRIT_ALLOWLISTED_TEST_ENV=allowed"));
        assert!(output.stdout.contains("REWRIT_RUNTIME_TEST_ENV=runtime"));
        assert!(output.stdout.contains("REWRIT_NETWORK_MODE=inherit"));
        assert!(!output.stdout.contains("REWRIT_BLOCKED_TEST_ENV=blocked"));

        {
            let _guard = ENV_LOCK.lock().expect("env lock");
            std::env::remove_var("REWRIT_ALLOWLISTED_TEST_ENV");
            std::env::remove_var("REWRIT_BLOCKED_TEST_ENV");
        }
    }

    #[tokio::test]
    async fn temp_dir_is_created_and_injected_into_runtime_env() {
        let temp = tempfile::tempdir().expect("tempdir");
        let runtime_temp = temp.path().join("runtime-tmp");
        let runner = ProcessRunner::new(RunnerConfig::default(), &SecurityConfig::default());
        let process = RuntimeProcess {
            command: vec!["/usr/bin/env".to_string()],
            cwd: std::env::current_dir().expect("cwd"),
            env: BTreeMap::new(),
            temp_dir: Some(runtime_temp.clone()),
            timeout_ms: 30_000,
            max_stdout_bytes: 1_048_576,
            max_stderr_bytes: 1_048_576,
            sandbox: SandboxConfig::default(),
        };

        let output = runner.run(&process).await.expect("run");

        assert!(runtime_temp.is_dir());
        assert!(output
            .stdout
            .contains(&format!("TMPDIR={}", runtime_temp.display())));
        assert!(output
            .stdout
            .contains(&format!("TEMP={}", runtime_temp.display())));
    }

    #[test]
    fn sandbox_wraps_command_for_container_runtime() {
        let process = RuntimeProcess {
            command: vec!["php".to_string(), "run.php".to_string()],
            cwd: PathBuf::from("/repo/app"),
            env: BTreeMap::from([(
                "REWRIT_EVENTS_PATH".to_string(),
                "/repo/.rewrit/events.ndjson".to_string(),
            )]),
            temp_dir: Some(PathBuf::from("/repo/.rewrit/tmp/run")),
            timeout_ms: 30_000,
            max_stdout_bytes: 1_048_576,
            max_stderr_bytes: 1_048_576,
            sandbox: SandboxConfig {
                enabled: true,
                image: Some("php:8.3-cli".to_string()),
                network: SandboxNetwork::Disabled,
                ..SandboxConfig::default()
            },
        };

        let (program, args) = command_with_optional_sandbox(&process, NetworkMode::Disabled);

        assert_eq!(program, "docker");
        assert!(args.contains(&"--network=none".to_string()));
        assert!(args.contains(&"php:8.3-cli".to_string()));
        assert!(args.contains(&"php".to_string()));
        assert!(args.contains(&"run.php".to_string()));
        assert!(args.contains(&"REWRIT_EVENTS_PATH=/repo/.rewrit/events.ndjson".to_string()));
    }
}
