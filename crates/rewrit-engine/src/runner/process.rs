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
}

impl ProcessRunner {
    #[must_use]
    pub fn new(runner: RunnerConfig, security: &SecurityConfig) -> Self {
        Self {
            runner,
            redactor: Redactor::new(&security.redact_patterns),
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

    pub async fn run(&self, process: &RuntimeProcess) -> Result<ProcessOutput, ProcessError> {
        let Some((program, args)) = process.command.split_first() else {
            return Err(ProcessError::EmptyCommand);
        };

        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(&process.cwd)
            .envs(&process.env)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(self.runner.kill_process_tree);

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
