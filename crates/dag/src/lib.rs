#![forbid(unsafe_code)]
//! Eksekutor pipeline DAG `workflow.toml` dan fan-out arena (Fase 4).

use latch_core::model::PipelineStatus;
use thiserror::Error;

/// Kesalahan eksekutor pipeline DAG.
#[derive(Debug, Error)]
pub enum DagError {
    /// Eksekutor DAG dijadwalkan untuk v1.0.
    #[error("DAG pipeline executor is scheduled for v1.0")]
    NotImplemented,
}

/// Jenis node yang tersedia pada `workflow.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowNodeKind {
    /// Node eksekusi agen AI.
    Agent,
    /// Node fan-out paralel ke beberapa profil.
    Fanout,
    /// Node deterministik perintah shell/uji.
    Shell,
}

/// Status awal pipeline sebelum eksekusi dimulai.
pub const fn initial_pipeline_status() -> PipelineStatus {
    PipelineStatus::Pending
}

#[cfg(test)]
mod tests {
    use latch_core::model::PipelineStatus;

    use super::{initial_pipeline_status, WorkflowNodeKind};

    #[test]
    fn distinct_node_kinds() {
        assert_ne!(WorkflowNodeKind::Agent, WorkflowNodeKind::Shell);
        assert_eq!(initial_pipeline_status(), PipelineStatus::Pending);
    }
}
