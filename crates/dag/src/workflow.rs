//! Parser dan validator `workflow.toml` sesuai `docs/WORKFLOW_SPEC.md` (T-401).

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::DagError;

/// Definisi lengkap satu berkas `workflow.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowSpec {
    /// Metadata dan batas utama workflow.
    pub workflow: WorkflowMeta,
    /// Pengaturan circuit breaker opsional khusus workflow.
    #[serde(default)]
    pub circuit_breaker: WorkflowCircuitBreaker,
    /// Daftar langkah DAG.
    pub steps: Vec<StepSpec>,
}

/// Seksi `[workflow]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowMeta {
    /// Nama unik workflow (`[a-z0-9-_]+`).
    pub name: String,
    /// Anggaran total pipeline USD (`None` = tak terbatas).
    #[serde(default)]
    pub max_budget_usd: Option<f64>,
    /// Batas node paralel (bawaan 2).
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
    /// Ref dasar opsional untuk worktree.
    #[serde(default)]
    pub base_ref: Option<String>,
    /// Deskripsi dokumentasi opsional.
    #[serde(default)]
    pub description: Option<String>,
}

fn default_max_parallel() -> usize {
    2
}

/// Seksi `[circuit_breaker]` pada `workflow.toml`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkflowCircuitBreaker {
    /// Jumlah error beruntun sebelum trip.
    pub max_consecutive_errors: Option<u32>,
    /// Batas biaya per sesi.
    pub max_session_budget_usd: Option<f64>,
    /// Ambang idle hibernasi (menit).
    pub idle_hibernate_minutes: Option<u64>,
}

/// Jenis node pada `[[steps]]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeKind {
    /// Node eksekusi agen AI.
    Agent,
    /// Node fan-out paralel ke beberapa profil.
    Fanout,
    /// Node deterministik perintah shell/uji.
    Shell,
}

/// Strategi pemilihan pemenang pada node `fanout` (`T-406`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectStrategy {
    /// Pemilihan manual oleh manusia.
    #[default]
    Manual,
    /// Kandidat lolos dengan biaya USD terendah.
    CheapestPassing,
    /// Kandidat lolos dengan durasi tercepat.
    FastestPassing,
    /// Kandidat lolos dengan jumlah baris diff terkecil.
    SmallestDiffPassing,
}

/// Definisi satu node langkah pada `[[steps]]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepSpec {
    /// ID unik langkah (`[a-z0-9_]+`).
    pub id: String,
    /// Jenis langkah (`agent`, `fanout`, `shell`).
    #[serde(rename = "type")]
    pub kind: WorkflowNodeKind,
    /// Daftar ID langkah prasyarat.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Nama profil agen (wajib untuk `agent`).
    #[serde(default)]
    pub profile: Option<String>,
    /// Daftar profil paralel (wajib >= 2 untuk `fanout`).
    #[serde(default)]
    pub profiles: Vec<String>,
    /// Template prompt untuk `agent` / `fanout`.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Perintah shell (wajib untuk `shell`).
    #[serde(default)]
    pub command: Option<String>,
    /// Perintah verifikasi opsional per kandidat `fanout`.
    #[serde(default)]
    pub verify_command: Option<String>,
    /// Strategi pemilihan pemenang `fanout`.
    #[serde(default)]
    pub select: SelectStrategy,
    /// Target retry gate saat `shell` gagal (`T-404`).
    #[serde(default)]
    pub on_fail_retry_step: Option<String>,
    /// Batas retry maksimal (bawaan 0).
    #[serde(default)]
    pub max_retries: u32,
    /// Kode keluar sukses yang diharapkan untuk `shell` (bawaan 0).
    #[serde(default)]
    pub expect_exit: i32,
    /// Batas byte stdout/stderr umpan balik retry (bawaan 16 KiB).
    #[serde(default = "default_feedback_max_bytes")]
    pub feedback_max_bytes: usize,
    /// Variabel lingkungan tambahan node `shell`.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

fn default_feedback_max_bytes() -> usize {
    16 * 1024
}

/// Mem-parse dan memvalidasi `workflow.toml`, mengembalikan `WorkflowSpec` + urutan topologi valid.
pub fn parse_and_validate_workflow(src: &str) -> Result<(WorkflowSpec, Vec<String>), DagError> {
    let spec: WorkflowSpec = toml::from_str(src).map_err(|e| DagError::Parse(e.to_string()))?;
    let order = validate_workflow(&spec)?;
    Ok((spec, order))
}

/// Memvalidasi aturan semantik dan keasiklikan graf `WorkflowSpec` (`WORKFLOW_SPEC.md` §5).
pub fn validate_workflow(spec: &WorkflowSpec) -> Result<Vec<String>, DagError> {
    if spec.workflow.name.is_empty()
        || !spec
            .workflow
            .name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
    {
        return Err(DagError::Validation(format!(
            "invalid workflow.name '{}': must match [a-z0-9-_]+",
            spec.workflow.name
        )));
    }
    if spec.workflow.max_parallel == 0 {
        return Err(DagError::Validation(
            "workflow.max_parallel must be >= 1".to_string(),
        ));
    }
    if spec.steps.is_empty() {
        return Err(DagError::Validation(
            "workflow must define at least one step".to_string(),
        ));
    }

    let mut by_id: HashMap<&str, &StepSpec> = HashMap::new();
    for step in &spec.steps {
        if step.id.is_empty()
            || !step
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(DagError::Validation(format!(
                "invalid step id '{}': must match [a-z0-9_]+",
                step.id
            )));
        }
        if by_id.insert(step.id.as_str(), step).is_some() {
            return Err(DagError::Validation(format!(
                "duplicate step id '{}'",
                step.id
            )));
        }
        match step.kind {
            WorkflowNodeKind::Agent => {
                if step.profile.as_deref().is_none_or(|p| p.trim().is_empty()) {
                    return Err(DagError::Validation(format!(
                        "agent step '{}' requires 'profile'",
                        step.id
                    )));
                }
                if step.prompt.as_deref().is_none_or(|p| p.trim().is_empty()) {
                    return Err(DagError::Validation(format!(
                        "agent step '{}' requires 'prompt'",
                        step.id
                    )));
                }
            }
            WorkflowNodeKind::Fanout => {
                if step.profiles.len() < 2 {
                    return Err(DagError::Validation(format!(
                        "fanout step '{}' requires at least 2 profiles",
                        step.id
                    )));
                }
                if step.prompt.as_deref().is_none_or(|p| p.trim().is_empty()) {
                    return Err(DagError::Validation(format!(
                        "fanout step '{}' requires 'prompt'",
                        step.id
                    )));
                }
            }
            WorkflowNodeKind::Shell => {
                if step.command.as_deref().is_none_or(|c| c.trim().is_empty()) {
                    return Err(DagError::Validation(format!(
                        "shell step '{}' requires 'command'",
                        step.id
                    )));
                }
                if step.max_retries > 0 && step.on_fail_retry_step.is_none() {
                    return Err(DagError::Validation(format!(
                        "shell step '{}' sets max_retries > 0 without on_fail_retry_step",
                        step.id
                    )));
                }
            }
        }
    }

    // Bangun adjacency & in-degree untuk toposort Kahn (deteksi siklus)
    let mut in_degree: HashMap<&str, usize> =
        spec.steps.iter().map(|s| (s.id.as_str(), 0)).collect();
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for step in &spec.steps {
        for dep in &step.depends_on {
            if !by_id.contains_key(dep.as_str()) {
                return Err(DagError::Validation(format!(
                    "step '{}' depends on unknown step '{dep}'",
                    step.id
                )));
            }
            children
                .entry(dep.as_str())
                .or_default()
                .push(step.id.as_str());
            if let Some(d) = in_degree.get_mut(step.id.as_str()) {
                *d += 1;
            }
        }
    }

    let mut queue: VecDeque<&str> = spec
        .steps
        .iter()
        .filter(|s| in_degree.get(s.id.as_str()).copied().unwrap_or(0) == 0)
        .map(|s| s.id.as_str())
        .collect();
    let mut order = Vec::with_capacity(spec.steps.len());

    while let Some(node) = queue.pop_front() {
        order.push(node.to_string());
        if let Some(nexts) = children.get(node) {
            for &next in nexts {
                if let Some(d) = in_degree.get_mut(next) {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        queue.push_back(next);
                    }
                }
            }
        }
    }

    if order.len() != spec.steps.len() {
        return Err(DagError::CycleDetected);
    }

    // Pastikan on_fail_retry_step merujuk leluhur yang sah
    for step in &spec.steps {
        if let Some(ref retry_target) = step.on_fail_retry_step {
            if !is_ancestor(retry_target, &step.id, &by_id) {
                return Err(DagError::Validation(format!(
                    "step '{}' on_fail_retry_step '{}' must be an ancestor",
                    step.id, retry_target
                )));
            }
        }
    }

    Ok(order)
}

fn is_ancestor(target: &str, from: &str, by_id: &HashMap<&str, &StepSpec>) -> bool {
    let mut stack = vec![from];
    let mut visited = HashSet::new();
    while let Some(cur) = stack.pop() {
        if !visited.insert(cur) {
            continue;
        }
        if let Some(step) = by_id.get(cur) {
            for dep in &step.depends_on {
                if dep == target {
                    return true;
                }
                stack.push(dep.as_str());
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::parse_and_validate_workflow;

    #[test]
    fn validates_shipped_example_workflow_toml() {
        let (spec, order) =
            parse_and_validate_workflow(include_str!("../../../examples/workflow.toml")).unwrap();
        assert_eq!(spec.workflow.name, "arena-implement-and-verify");
        assert_eq!(
            order,
            vec!["arena_impl", "review_and_polish", "final_verify"]
        );
    }
}
