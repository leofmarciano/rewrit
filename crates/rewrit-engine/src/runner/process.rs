use crate::discovery::manifest::{RunnerConfig, RuntimeConfig, SecurityConfig};
use crate::runner::env::{truncate, Redactor};
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
    pub timeout_ms: u64,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
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
}

impl ProcessRunner {
    #[must_use]
    pub fn new(runner: RunnerConfig, security: &SecurityConfig) -> Self {
        Self {
            runner,
            redactor: Redactor::new(&security.redact_patterns),
            env_allowlist: security.env_allowlist.clone(),
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
            timeout_ms: runtime
                .timeout_ms
                .or(self.runner.default_timeout_ms)
                .unwrap_or(30_000),
            max_stdout_bytes: self.runner.max_stdout_bytes.unwrap_or(1_048_576),
            max_stderr_bytes: self.runner.max_stderr_bytes.unwrap_or(1_048_576),
        }
    }

    pub fn apply_environment(&self, command: &mut Command, runtime_env: &BTreeMap<String, String>) {
        if !self.env_allowlist.is_empty() {
            command.env_clear();
            for (key, value) in std::env::vars() {
                if env_allowed(&key, &self.env_allowlist) {
                    command.env(key, value);
                }
            }
        }
        command.envs(runtime_env);
    }

    pub async fn run(&self, process: &RuntimeProcess) -> Result<ProcessOutput, ProcessError> {
        let Some((program, args)) = process.command.split_first() else {
            return Err(ProcessError::EmptyCommand);
        };

        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(&process.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(self.runner.kill_process_tree);
        self.apply_environment(&mut command, &process.env);

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

fn env_allowed(key: &str, allowlist: &[String]) -> bool {
    allowlist.iter().any(|entry| {
        if let Some(prefix) = entry.strip_suffix('*') {
            key.starts_with(prefix)
        } else {
            key == entry
        }
    })
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
            timeout_ms: 30_000,
            max_stdout_bytes: 1_048_576,
            max_stderr_bytes: 1_048_576,
        };

        let output = runner.run(&process).await.expect("run");

        assert!(output
            .stdout
            .contains("REWRIT_ALLOWLISTED_TEST_ENV=allowed"));
        assert!(output.stdout.contains("REWRIT_RUNTIME_TEST_ENV=runtime"));
        assert!(!output.stdout.contains("REWRIT_BLOCKED_TEST_ENV=blocked"));

        {
            let _guard = ENV_LOCK.lock().expect("env lock");
            std::env::remove_var("REWRIT_ALLOWLISTED_TEST_ENV");
            std::env::remove_var("REWRIT_BLOCKED_TEST_ENV");
        }
    }
}
