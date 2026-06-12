//! AI iteration harness.
//!
//! An *iteration* is a short-lived, sandboxed editing session against a
//! skill's source tree. An AI agent (or a human via API) does roughly:
//!
//! 1. Open a job:        `Harness::open(skill, base_version, agent, intent)`
//! 2. Pull current files: `Harness::list_files / read_file`
//! 3. Push patches:       `Harness::apply_patch`
//! 4. Run tests:          `Harness::run_tests`
//! 5. Submit:             `Harness::submit` → produces a `VersionDraft`
//!    (and the API layer wraps it in a `VersionProposal`).
//!
//! The crate is deliberately storage-agnostic: it talks to a
//! `ObjectStore` for the packaged base version, and uses an OS temp
//! dir as the workspace. State transitions are pure functions on
//! `IterationJob` — IO and persistence are pushed to the caller.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::process::Command;
use tracing::{debug, instrument};
use uuid::Uuid;

use skillhub_domain::iteration::{
    IterationJob, IterationPatch, IterationState, IterationTestRun, PatchOp,
};

pub mod sandbox;
pub use sandbox::SandboxLimits;

#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    #[error("invalid state transition: {0:?} → {1:?}")]
    InvalidTransition(IterationState, IterationState),
    #[error("workspace path escape: {0}")]
    PathEscape(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("internal: {0}")]
    Other(String),
}

pub type HarnessResult<T> = Result<T, HarnessError>;

/// Where workspaces live on disk. Defaults to `$TMP/skillhub-harness/<job>`.
#[derive(Debug, Clone)]
pub struct HarnessConfig {
    pub root: PathBuf,
    pub default_limits: SandboxLimits,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            root: std::env::temp_dir().join("skillhub-harness"),
            default_limits: SandboxLimits::default(),
        }
    }
}

/// A change request the agent sends to the harness. Not serde —
/// the HTTP layer decodes wire formats into this type.
#[derive(Debug, Clone)]
pub struct PatchInput {
    pub path: String,
    pub op: PatchOp,
    /// For `Write`: file content. For `Rename`: target path.
    pub data: Option<bytes::Bytes>,
    pub new_path: Option<String>,
}

/// Output of running a test command inside the workspace sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestOutcome {
    pub command: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

#[derive(Clone)]
pub struct Harness {
    cfg: Arc<HarnessConfig>,
}

impl Harness {
    pub fn new(cfg: HarnessConfig) -> Self {
        Self { cfg: Arc::new(cfg) }
    }

    fn workspace_dir(&self, job_id: Uuid) -> PathBuf {
        self.cfg.root.join(job_id.to_string())
    }

    pub fn ensure_workspace(&self, job_id: Uuid) -> HarnessResult<PathBuf> {
        let dir = self.workspace_dir(job_id);
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// State machine helper. Returns the *new* job with timestamps + state
    /// updated; persistence is the caller's responsibility.
    pub fn transition(
        &self,
        mut job: IterationJob,
        to: IterationState,
    ) -> HarnessResult<IterationJob> {
        if !valid_transition(job.state, to) {
            return Err(HarnessError::InvalidTransition(job.state, to));
        }
        let now = Utc::now();
        match to {
            IterationState::Running if job.started_at.is_none() => job.started_at = Some(now),
            IterationState::Succeeded
            | IterationState::Failed
            | IterationState::Cancelled
            | IterationState::Submitted => {
                if job.finished_at.is_none() {
                    job.finished_at = Some(now);
                }
            }
            _ => {}
        }
        job.state = to;
        Ok(job)
    }

    /// Apply one patch to the workspace and produce a record to persist.
    #[instrument(skip(self, input), fields(job_id = %job_id, path = %input.path))]
    pub async fn apply_patch(
        &self,
        job_id: Uuid,
        seq: i32,
        input: PatchInput,
    ) -> HarnessResult<IterationPatch> {
        let root = self.ensure_workspace(job_id)?;
        let target = safe_join(&root, &input.path)?;
        let (content_sha, size) = match input.op {
            PatchOp::Write => {
                let data = input.data.ok_or_else(|| {
                    HarnessError::Other("write patch missing data".into())
                })?;
                if let Some(parent) = target.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(&target, &data).await?;
                (Some(sha256_hex(&data)), Some(data.len() as i64))
            }
            PatchOp::Delete => {
                if target.exists() {
                    tokio::fs::remove_file(&target).await?;
                }
                (None, None)
            }
            PatchOp::Rename => {
                let new_path = input.new_path.as_ref().ok_or_else(|| {
                    HarnessError::Other("rename patch missing new_path".into())
                })?;
                let dst = safe_join(&root, new_path)?;
                if let Some(parent) = dst.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::rename(&target, &dst).await?;
                (None, None)
            }
        };
        Ok(IterationPatch {
            id: Uuid::new_v4(),
            job_id,
            seq,
            path: input.path,
            op: input.op,
            new_path: input.new_path,
            content_sha256: content_sha,
            size_bytes: size,
            applied_at: Utc::now(),
        })
    }

    /// Run a single test command. The command is parsed shell-free
    /// (whitespace split) — agents pass argv-style commands.
    #[instrument(skip(self, limits), fields(job_id = %job_id, command = command))]
    pub async fn run_tests(
        &self,
        job_id: Uuid,
        command: &str,
        limits: Option<SandboxLimits>,
    ) -> HarnessResult<(TestOutcome, IterationTestRun)> {
        let limits = limits.unwrap_or(self.cfg.default_limits);
        let root = self.ensure_workspace(job_id)?;
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(HarnessError::Other("empty command".into()));
        }
        let mut cmd = Command::new(parts[0]);
        cmd.args(&parts[1..])
            .current_dir(&root)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", root.to_string_lossy().to_string())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let started = std::time::Instant::now();
        let mut child = cmd.spawn()?;
        let timed_out;
        let exit_code;
        let stdout;
        let stderr;
        match tokio::time::timeout(Duration::from_secs(limits.wall_seconds), child.wait_with_output()).await {
            Ok(Ok(output)) => {
                timed_out = false;
                exit_code = output.status.code().unwrap_or(-1);
                stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            }
            Ok(Err(e)) => return Err(HarnessError::Io(e)),
            Err(_) => {
                timed_out = true;
                exit_code = -1;
                stdout = String::new();
                stderr = "timeout".into();
            }
        }
        let elapsed = started.elapsed();
        let outcome = TestOutcome {
            command: command.to_string(),
            exit_code,
            duration_ms: elapsed.as_millis() as u64,
            stdout,
            stderr,
            timed_out,
        };
        debug!(?outcome, "iteration test finished");
        let record = IterationTestRun {
            id: Uuid::new_v4(),
            job_id,
            command: command.to_string(),
            exit_code: Some(exit_code),
            duration_ms: Some(elapsed.as_millis() as i32),
            stdout_uri: None,
            stderr_uri: None,
            started_at: Utc::now() - chrono::Duration::from_std(elapsed).unwrap_or_default(),
            finished_at: Some(Utc::now()),
        };
        Ok((outcome, record))
    }

    /// Walk the workspace and emit `{path: bytes}` for everything inside.
    /// Used at submit-time to package a draft.
    pub async fn snapshot(&self, job_id: Uuid) -> HarnessResult<HashMap<String, Vec<u8>>> {
        let root = self.ensure_workspace(job_id)?;
        let mut out = HashMap::new();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            let mut entries = tokio::fs::read_dir(&dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let ft = entry.file_type().await?;
                if ft.is_dir() {
                    stack.push(path);
                } else if ft.is_file() {
                    let rel = path
                        .strip_prefix(&root)
                        .map_err(|_| HarnessError::PathEscape(path.display().to_string()))?
                        .to_string_lossy()
                        .into_owned();
                    let data = tokio::fs::read(&path).await?;
                    out.insert(rel, data);
                }
            }
        }
        Ok(out)
    }
}

fn valid_transition(from: IterationState, to: IterationState) -> bool {
    use IterationState::*;
    matches!(
        (from, to),
        (Queued, Running)
            | (Queued, Cancelled)
            | (Running, Succeeded)
            | (Running, Failed)
            | (Running, Cancelled)
            | (Succeeded, Submitted)
    )
}

fn safe_join(root: &Path, rel: &str) -> HarnessResult<PathBuf> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() || rel_path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(HarnessError::PathEscape(rel.into()));
    }
    Ok(root.join(rel_path))
}

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_escape() {
        let r = safe_join(Path::new("/tmp/x"), "../etc/passwd");
        assert!(r.is_err());
    }

    #[test]
    fn allows_relative_subpath() {
        assert!(safe_join(Path::new("/tmp/x"), "src/lib.rs").is_ok());
    }

    #[test]
    fn transitions_obey_state_machine() {
        let h = Harness::new(HarnessConfig::default());
        let job = IterationJob {
            id: Uuid::new_v4(),
            skill_id: Uuid::new_v4(),
            base_version_id: None,
            started_by: Uuid::new_v4(),
            agent: "test".into(),
            intent: "test".into(),
            state: IterationState::Queued,
            workspace_key: "k".into(),
            log_uri: None,
            error: None,
            submitted_proposal: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
        };
        let running = h.transition(job.clone(), IterationState::Running).unwrap();
        assert_eq!(running.state, IterationState::Running);
        assert!(running.started_at.is_some());
        // Cannot jump straight to submitted from running.
        assert!(h.transition(running.clone(), IterationState::Submitted).is_err());
        let succ = h.transition(running, IterationState::Succeeded).unwrap();
        let sub = h.transition(succ, IterationState::Submitted).unwrap();
        assert_eq!(sub.state, IterationState::Submitted);
    }
}
