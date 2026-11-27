//! Eksekutor DAG dengan retry gate, fan-out arena, aturan pemenang, anggaran pipeline,
//! dan serialisasi state untuk pemulihan pasca-restart (T-402, T-404..T-408).

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use latch_core::model::PipelineStatus;
use serde::{Deserialize, Serialize};

use crate::template::{render_strict_template, truncate_output, DEFAULT_OUTPUT_MAX_BYTES};
use crate::workflow::{
    validate_workflow, SelectStrategy, StepSpec, WorkflowNodeKind, WorkflowSpec,
};
use crate::DagError;

/// Hasil eksekusi satu langkah `agent` atau `shell`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionOutput {
    /// Kode keluar (`0` = sukses).
    pub exit_code: i32,
    /// Keluaran utama (stdout / final message).
    pub output: String,
    /// Keluaran error (stderr) untuk umpan balik retry.
    pub stderr: String,
    /// Diff ringkas jika tersedia.
    pub diff: String,
    /// Jumlah baris diff berubah.
    pub diff_lines: u32,
    /// Estimasi biaya USD langkah ini.
    pub cost_usd: f64,
    /// Durasi eksekusi (ms).
    pub duration_ms: u64,
    /// ID sesi opsional bila dijalankan lewat agen.
    pub session_id: Option<String>,
}

/// Metrik satu kandidat pada matriks Fan-Out Arena (`T-405`, FR-20).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaCandidate {
    /// Nama profil kandidat.
    pub profile: String,
    /// ID sesi terkait jika ada.
    pub session_id: Option<String>,
    /// Apakah lulus verifikasi.
    pub passed: bool,
    /// Durasi eksekusi (ms).
    pub duration_ms: u64,
    /// Jumlah baris diff.
    pub diff_lines: u32,
    /// Estimasi biaya USD.
    pub cost_usd: f64,
    /// Output terpotong kandidat.
    pub output: String,
}

/// State serial satu langkah di dalam `dag_state_json` (`T-408`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRuntimeState {
    /// Status langkah (`succeeded`, `failed`, `skipped`).
    pub status: String,
    /// Jumlah percobaan yang sudah berjalan.
    pub attempts: u32,
    /// Output terpotong terakhir.
    pub output: String,
    /// Diff terakhir.
    pub diff: String,
    /// Kandidat arena (khusus node `fanout`).
    #[serde(default)]
    pub candidates: Vec<ArenaCandidate>,
    /// Profil pemenang terpilih (khusus node `fanout`).
    #[serde(default)]
    pub winner: Option<String>,
}

/// State runtime keseluruhan pipeline yang dapat diserialisasi ke SQLite `dag_state_json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagRuntimeState {
    /// Nama workflow.
    pub workflow_name: String,
    /// Status akhir/berjalan pipeline.
    pub status: PipelineStatus,
    /// Total biaya USD kumulatif seluruh langkah.
    pub total_cost_usd: f64,
    /// Alasan gagal/trip jika ada.
    pub reason: Option<String>,
    /// State per langkah berdasarkan step ID.
    pub steps: HashMap<String, StepRuntimeState>,
}

/// Memilih pemenang kandidat arena berdasarkan strategi `SelectStrategy` (`T-406`).
pub fn select_arena_winner(
    candidates: &[ArenaCandidate],
    strategy: SelectStrategy,
) -> Option<&ArenaCandidate> {
    let passing: Vec<&ArenaCandidate> = candidates.iter().filter(|c| c.passed).collect();
    match strategy {
        SelectStrategy::Manual => passing.first().copied().or_else(|| candidates.first()),
        SelectStrategy::CheapestPassing => passing.into_iter().min_by(|a, b| {
            a.cost_usd
                .partial_cmp(&b.cost_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        SelectStrategy::FastestPassing => passing.into_iter().min_by_key(|c| c.duration_ms),
        SelectStrategy::SmallestDiffPassing => passing.into_iter().min_by_key(|c| c.diff_lines),
    }
}

/// Abstraksi runner langkah agar `latch-dag` independen dari runtime daemon/PTY.
pub trait StepRunner: Send + Sync {
    /// Menjalankan satu profil agen dengan prompt ter-render.
    fn run_agent<'a>(
        &'a self,
        step_id: &'a str,
        profile: &'a str,
        rendered_prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<StepExecutionOutput, DagError>> + Send + 'a>>;

    /// Menjalankan perintah shell deterministik.
    fn run_shell<'a>(
        &'a self,
        step_id: &'a str,
        command: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<StepExecutionOutput, DagError>> + Send + 'a>>;
}

/// Mengeksekusi seluruh graf `WorkflowSpec` lengkap dengan retry gate, fan-out arena, dan anggaran pipeline.
pub async fn execute_workflow(
    spec: &WorkflowSpec,
    task: &str,
    project_name: &str,
    default_branch: &str,
    runner: &dyn StepRunner,
) -> Result<DagRuntimeState, DagError> {
    let order = validate_workflow(spec)?;
    let by_id: HashMap<&str, &StepSpec> = spec.steps.iter().map(|s| (s.id.as_str(), s)).collect();

    let mut state = DagRuntimeState {
        workflow_name: spec.workflow.name.clone(),
        status: PipelineStatus::Running,
        total_cost_usd: 0.0,
        reason: None,
        steps: HashMap::new(),
    };

    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert("task".to_string(), task.to_string());
    vars.insert("project.name".to_string(), project_name.to_string());
    vars.insert(
        "project.default_branch".to_string(),
        default_branch.to_string(),
    );

    let mut retries_used: HashMap<String, u32> = HashMap::new();
    let mut idx = 0usize;

    while idx < order.len() {
        let step_id = &order[idx];
        let Some(step) = by_id.get(step_id.as_str()).copied() else {
            return Err(DagError::Validation(format!("missing step '{step_id}'")));
        };

        if let Some(max_budget) = spec.workflow.max_budget_usd {
            if state.total_cost_usd >= max_budget {
                state.status = PipelineStatus::Tripped;
                state.reason = Some(format!(
                    "pipeline cost ${:.4} exceeded max_budget_usd ${:.4}",
                    state.total_cost_usd, max_budget
                ));
                return Ok(state);
            }
        }

        match step.kind {
            WorkflowNodeKind::Agent => {
                let profile = step.profile.as_deref().unwrap_or("");
                let prompt_tpl = step.prompt.as_deref().unwrap_or("");
                let rendered = render_strict_template(prompt_tpl, &vars)?;
                let out = runner.run_agent(&step.id, profile, &rendered).await?;
                state.total_cost_usd += out.cost_usd.max(0.0);

                let truncated = truncate_output(&out.output, DEFAULT_OUTPUT_MAX_BYTES);
                let ok = out.exit_code == 0;
                let attempts = state
                    .steps
                    .get(&step.id)
                    .map_or(1, |prev| prev.attempts.saturating_add(1));
                state.steps.insert(
                    step.id.clone(),
                    StepRuntimeState {
                        status: if ok { "succeeded" } else { "failed" }.to_string(),
                        attempts,
                        output: truncated.clone(),
                        diff: out.diff.clone(),
                        candidates: Vec::new(),
                        winner: None,
                    },
                );
                vars.insert(format!("steps.{}.output", step.id), truncated);
                vars.insert(
                    format!("steps.{}.status", step.id),
                    if ok { "succeeded" } else { "failed" }.to_string(),
                );
                vars.insert(format!("steps.{}.diff", step.id), out.diff);

                if !ok {
                    state.status = PipelineStatus::Failed;
                    state.reason = Some(format!("agent step '{}' failed", step.id));
                    return Ok(state);
                }
            }
            WorkflowNodeKind::Fanout => {
                let prompt_tpl = step.prompt.as_deref().unwrap_or("");
                let rendered = render_strict_template(prompt_tpl, &vars)?;
                let mut candidates = Vec::with_capacity(step.profiles.len());

                for profile in &step.profiles {
                    let out = runner.run_agent(&step.id, profile, &rendered).await?;
                    state.total_cost_usd += out.cost_usd.max(0.0);
                    let mut passed = out.exit_code == 0;
                    if passed {
                        if let Some(ref verify_cmd) = step.verify_command {
                            let ver = runner.run_shell(&step.id, verify_cmd).await?;
                            passed = ver.exit_code == 0;
                        }
                    }
                    candidates.push(ArenaCandidate {
                        profile: profile.clone(),
                        session_id: out.session_id,
                        passed,
                        duration_ms: out.duration_ms,
                        diff_lines: out.diff_lines,
                        cost_usd: out.cost_usd,
                        output: truncate_output(&out.output, DEFAULT_OUTPUT_MAX_BYTES),
                    });
                }

                let winner = select_arena_winner(&candidates, step.select).cloned();
                let winner_output = winner
                    .as_ref()
                    .map(|w| w.output.clone())
                    .unwrap_or_default();
                let winner_profile = winner.as_ref().map(|w| w.profile.clone());
                let ok = winner.is_some();

                let attempts = state
                    .steps
                    .get(&step.id)
                    .map_or(1, |prev| prev.attempts.saturating_add(1));
                state.steps.insert(
                    step.id.clone(),
                    StepRuntimeState {
                        status: if ok { "succeeded" } else { "failed" }.to_string(),
                        attempts,
                        output: winner_output.clone(),
                        diff: String::new(),
                        candidates,
                        winner: winner_profile,
                    },
                );
                vars.insert(format!("steps.{}.output", step.id), winner_output);
                vars.insert(
                    format!("steps.{}.status", step.id),
                    if ok { "succeeded" } else { "failed" }.to_string(),
                );
                vars.insert(format!("steps.{}.diff", step.id), String::new());

                if !ok {
                    state.status = PipelineStatus::Failed;
                    state.reason = Some(format!(
                        "fanout step '{}' had no passing candidate",
                        step.id
                    ));
                    return Ok(state);
                }
            }
            WorkflowNodeKind::Shell => {
                let cmd = step.command.as_deref().unwrap_or("");
                let out = runner.run_shell(&step.id, cmd).await?;
                let ok = out.exit_code == step.expect_exit;
                let truncated_out = truncate_output(&out.output, step.feedback_max_bytes);
                let truncated_err = truncate_output(&out.stderr, step.feedback_max_bytes);

                let attempts = state
                    .steps
                    .get(&step.id)
                    .map_or(1, |prev| prev.attempts.saturating_add(1));
                state.steps.insert(
                    step.id.clone(),
                    StepRuntimeState {
                        status: if ok { "succeeded" } else { "failed" }.to_string(),
                        attempts,
                        output: truncated_out.clone(),
                        diff: String::new(),
                        candidates: Vec::new(),
                        winner: None,
                    },
                );
                vars.insert(format!("steps.{}.output", step.id), truncated_out.clone());
                vars.insert(
                    format!("steps.{}.status", step.id),
                    if ok { "succeeded" } else { "failed" }.to_string(),
                );
                vars.insert(format!("steps.{}.diff", step.id), String::new());

                if !ok {
                    let used = retries_used.entry(step.id.clone()).or_insert(0);
                    if let Some(ref retry_target) = step.on_fail_retry_step {
                        if *used < step.max_retries {
                            *used += 1;
                            vars.insert("retry.attempt".to_string(), used.to_string());
                            vars.insert("retry.stdout".to_string(), truncated_out);
                            vars.insert("retry.stderr".to_string(), truncated_err);
                            if let Some(target_idx) = order.iter().position(|id| id == retry_target)
                            {
                                idx = target_idx;
                                continue;
                            }
                        }
                    }
                    state.status = PipelineStatus::Failed;
                    state.reason = Some(format!(
                        "shell step '{}' failed with exit code {}",
                        step.id, out.exit_code
                    ));
                    return Ok(state);
                }
            }
        }

        if let Some(max_budget) = spec.workflow.max_budget_usd {
            if state.total_cost_usd >= max_budget {
                state.status = PipelineStatus::Tripped;
                state.reason = Some(format!(
                    "pipeline cost ${:.4} exceeded max_budget_usd ${:.4}",
                    state.total_cost_usd, max_budget
                ));
                return Ok(state);
            }
        }

        idx += 1;
    }

    state.status = PipelineStatus::Succeeded;
    Ok(state)
}
