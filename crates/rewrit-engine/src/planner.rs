use rewrit_model::CaseId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub case_ids: Vec<CaseId>,
}

impl ExecutionPlan {
    #[must_use]
    pub fn all() -> Self {
        Self {
            case_ids: Vec::new(),
        }
    }
}
