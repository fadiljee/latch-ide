//! Integrasi `StepRunner` daemon untuk mengeksekusi langkah `agent` dan `shell` pada pipeline DAG (T-402..T-408).

use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

use latch_core::model::{Project, SessionStatus};
use latch_core::protocol::{SessionDiffParams, SessionRunParams};
use latch_dag::{DagError, StepExecutionOutput, StepRunner};
use tokio::process::Command;

use crate::session_manager::{DaemonEvent, SessionManager};

/// Implementasi `StepRunner` yang menjalankan langkah agen lewat `SessionManager` dan shell via `sh -c`.
pub struct DaemonStepRunner {
    manager: SessionManager,
    project: Project,
    pipeline_run_id: String,
    base_ref: Option<String>,
}

impl DaemonStepRunner {
    /// Membuat runner langkah untuk satu eksekusi pipeline.
    pub fn new(
        manager: SessionManager,
        project: Project,
        pipeline_run_id: String,
        base_ref: Option<String>,
    ) -> Self {
        Self {
            manager,
            project,
            pipeline_run_id,
            base_ref,
        }
    }
}

impl StepRunner for DaemonStepRunner {
    fn run_agent<'a>(
        &'a self,
        step_id: &'a str,
        profile: &'a str,
        rendered_prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<StepExecutionOutput, DagError>> + Send + 'a>> {
        let manager = self.manager.clone();
        let project_id = self.project.id.clone();
        let pipeline_run_id = self.pipeline_run_id.clone();
        let base_ref = self.base_ref.clone();
        let step_id = step_id.to_string();
        let profile = profile.to_string();
        let prompt = rendered_prompt.to_string();

        Box::pin(async move {
            let started = Instant::now();
            let mut events = manager.subscribe_events();
            let session = manager
                .run_session_with_pipeline(
                    SessionRunParams {
                        project: project_id,
                        profile,
                        task: prompt,
                        base_ref,
                        extra_args: Vec::new(),
                    },
                    Some(pipeline_run_id),
                    Some(step_id),
                )
                .await
                .map_err(|e| DagError::Execution(e.to_string()))?;

            // Tunggu sampai sesi mencapai status akhir (event-driven + fallback polling).
            let final_ses = loop {
                let current = manager
                    .get_session(&session.id)
                    .await
                    .map_err(|e| DagError::Execution(e.to_string()))?;
                if current.status.is_terminal() {
                    break current;
                }
                let _ = tokio::time::timeout(Duration::from_millis(150), async {
                    while let Ok(ev) = events.recv().await {
                        if let DaemonEvent::SessionStatusChanged { session_id, to, .. } = ev {
                            if session_id == session.id && to.is_terminal() {
                                break;
                            }
                        }
                    }
                })
                .await;
            };

            let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
            let output = manager
                .screen_snapshot(&final_ses.id)
                .await
                .map(|s| s.text)
                .unwrap_or_default();
            let (diff, diff_lines) = match manager
                .diff_session(SessionDiffParams {
                    session_id: final_ses.id.clone(),
                    file: None,
                    max_bytes: Some(32 * 1024),
                })
                .await
            {
                Ok(d) => {
                    let lines: u32 = d
                        .files
                        .iter()
                        .map(|f| f.additions.saturating_add(f.deletions))
                        .sum();
                    (d.patch, lines)
                }
                Err(_) => (String::new(), 0),
            };
            let cost_usd = manager
                .session_telemetry_summary(&final_ses.id)
                .await
                .map(|s| s.estimated_cost_usd)
                .unwrap_or(0.0);

            let exit_code = final_ses.exit_code.unwrap_or(match final_ses.status {
                SessionStatus::Completed => 0,
                _ => 1,
            });

            Ok(StepExecutionOutput {
                exit_code,
                output,
                stderr: String::new(),
                diff,
                diff_lines,
                cost_usd,
                duration_ms,
                session_id: Some(final_ses.id),
            })
        })
    }

    fn run_shell<'a>(
        &'a self,
        _step_id: &'a str,
        command: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<StepExecutionOutput, DagError>> + Send + 'a>> {
        let repo_path = self.project.repo_path.clone();
        let command = command.to_string();

        Box::pin(async move {
            let started = Instant::now();
            let out = Command::new("sh")
                .arg("-c")
                .arg(&command)
                .current_dir(&repo_path)
                .output()
                .await
                .map_err(|e| DagError::Execution(format!("failed running shell step: {e}")))?;

            let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
            let exit_code = out.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();

            Ok(StepExecutionOutput {
                exit_code,
                output: stdout,
                stderr,
                diff: String::new(),
                diff_lines: 0,
                cost_usd: 0.0,
                duration_ms,
                session_id: None,
            })
        })
    }
}
