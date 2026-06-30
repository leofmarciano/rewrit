use rewrit_model::{CaseId, Divergence, Observation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EngineEvent {
    RunStarted { run_id: String },
    CaseDiscovered { case_id: CaseId },
    CaseStarted { case_id: CaseId },
    ObservationReceived { observation: Observation },
    CaseCompared { case_id: CaseId, equivalent: bool },
    DivergenceFound { divergence: Divergence },
    RunFinished { run_id: String, exit_code: i32 },
}

