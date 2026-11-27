#![forbid(unsafe_code)]
//! Eksekutor pipeline DAG `workflow.toml`, templating strict, retry gate, dan fan-out arena (T-401..T-408).

pub mod executor;
pub mod template;
pub mod workflow;

use latch_core::model::PipelineStatus;
use thiserror::Error;

pub use executor::{
    execute_workflow, select_arena_winner, ArenaCandidate, DagRuntimeState, StepExecutionOutput,
    StepRunner, StepRuntimeState,
};
pub use template::{
    render_strict_template, truncate_output, DEFAULT_OUTPUT_MAX_BYTES, TRUNCATED_MARKER,
};
pub use workflow::{
    parse_and_validate_workflow, validate_workflow, SelectStrategy, StepSpec,
    WorkflowCircuitBreaker, WorkflowMeta, WorkflowNodeKind, WorkflowSpec,
};

/// Kesalahan parser, validator, atau eksekutor pipeline DAG.
#[derive(Debug, Error)]
pub enum DagError {
    /// Galat parsing TOML `workflow.toml`.
    #[error("invalid workflow.toml: {0}")]
    Parse(String),
    /// Galat validasi semantik workflow.
    #[error("workflow validation error: {0}")]
    Validation(String),
    /// Graf dependensi mengandung siklus.
    #[error("workflow dependency graph contains a cycle")]
    CycleDetected,
    /// Galat sintaks template.
    #[error("template error: {0}")]
    Template(String),
    /// Variabel template tidak dikenal pada mode strict.
    #[error("unknown template variable: '{0}'")]
    UnknownTemplateVar(String),
    /// Galat eksekusi langkah.
    #[error("step execution failed: {0}")]
    Execution(String),
}

/// Status awal pipeline sebelum eksekusi dimulai.
pub const fn initial_pipeline_status() -> PipelineStatus {
    PipelineStatus::Pending
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicU32, Ordering};

    use latch_core::model::PipelineStatus;

    use super::{
        execute_workflow, parse_and_validate_workflow, select_arena_winner, ArenaCandidate,
        DagError, SelectStrategy, StepExecutionOutput, StepRunner,
    };

    struct MockRunner {
        shell_calls: AtomicU32,
    }

    impl StepRunner for MockRunner {
        fn run_agent<'a>(
            &'a self,
            _step_id: &'a str,
            profile: &'a str,
            rendered_prompt: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<StepExecutionOutput, DagError>> + Send + 'a>>
        {
            let profile = profile.to_string();
            let prompt = rendered_prompt.to_string();
            Box::pin(async move {
                let (cost, dur, diff_lines) = match profile.as_str() {
                    "cheap" => (0.10, 500, 30),
                    "fast" => (0.40, 100, 40),
                    _ => (0.25, 250, 12),
                };
                Ok(StepExecutionOutput {
                    exit_code: 0,
                    output: format!("done:{profile}:{prompt}"),
                    stderr: String::new(),
                    diff: String::new(),
                    diff_lines,
                    cost_usd: cost,
                    duration_ms: dur,
                    session_id: Some(format!("ses-{profile}")),
                })
            })
        }

        fn run_shell<'a>(
            &'a self,
            _step_id: &'a str,
            _command: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<StepExecutionOutput, DagError>> + Send + 'a>>
        {
            let attempt = self.shell_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                Ok(StepExecutionOutput {
                    exit_code: if attempt == 0 { 1 } else { 0 },
                    output: "test output".to_string(),
                    stderr: if attempt == 0 {
                        "assertion failed on line 10".to_string()
                    } else {
                        String::new()
                    },
                    diff: String::new(),
                    diff_lines: 0,
                    cost_usd: 0.0,
                    duration_ms: 20,
                    session_id: None,
                })
            })
        }
    }

    #[test]
    fn rejects_cyclic_workflow() {
        let cyclic = r#"
[workflow]
name = "cyclic"

[[steps]]
id = "a"
type = "shell"
depends_on = ["b"]
command = "echo a"

[[steps]]
id = "b"
type = "shell"
depends_on = ["a"]
command = "echo b"
"#;
        let err = parse_and_validate_workflow(cyclic).unwrap_err();
        assert!(matches!(err, DagError::CycleDetected));
    }

    #[tokio::test]
    async fn executes_plan_fanout_verify_with_retry_gate_and_budget_trip() {
        let wf_toml = r#"
[workflow]
name = "feature-verify"
max_budget_usd = 5.0

[[steps]]
id = "plan"
type = "agent"
profile = "planner"
prompt = "Plan: {{task}}"

[[steps]]
id = "implement"
type = "fanout"
depends_on = ["plan"]
profiles = ["cheap", "fast"]
prompt = "Impl from {{steps.plan.output}}"
select = "cheapest_passing"

[[steps]]
id = "verify"
type = "shell"
depends_on = ["implement"]
command = "cargo test"
on_fail_retry_step = "implement"
max_retries = 2
"#;
        let (spec, order) = parse_and_validate_workflow(wf_toml).unwrap();
        assert_eq!(order, vec!["plan", "implement", "verify"]);

        let runner = MockRunner {
            shell_calls: AtomicU32::new(0),
        };
        let state = execute_workflow(&spec, "Add health", "demo", "main", &runner)
            .await
            .unwrap();
        assert_eq!(state.status, PipelineStatus::Succeeded);
        assert_eq!(state.steps["verify"].attempts, 2);
        assert_eq!(state.steps["implement"].winner.as_deref(), Some("cheap"));

        // Uji strategi pemilihan pemenang lainnya
        let cands = vec![
            ArenaCandidate {
                profile: "cheap".to_string(),
                session_id: None,
                passed: true,
                duration_ms: 500,
                diff_lines: 30,
                cost_usd: 0.10,
                output: String::new(),
            },
            ArenaCandidate {
                profile: "fast".to_string(),
                session_id: None,
                passed: true,
                duration_ms: 100,
                diff_lines: 5,
                cost_usd: 0.40,
                output: String::new(),
            },
        ];
        assert_eq!(
            select_arena_winner(&cands, SelectStrategy::FastestPassing)
                .unwrap()
                .profile,
            "fast"
        );
        assert_eq!(
            select_arena_winner(&cands, SelectStrategy::SmallestDiffPassing)
                .unwrap()
                .profile,
            "fast"
        );

        // Uji trip anggaran pipeline
        let mut tight = spec.clone();
        tight.workflow.max_budget_usd = Some(0.20);
        let runner2 = MockRunner {
            shell_calls: AtomicU32::new(0),
        };
        let tripped = execute_workflow(&tight, "Add health", "demo", "main", &runner2)
            .await
            .unwrap();
        assert_eq!(tripped.status, PipelineStatus::Tripped);
    }
}
