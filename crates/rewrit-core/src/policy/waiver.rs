use rewrit_model::{CaseId, Divergence, DivergenceKind, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waiver {
    pub case: CaseId,
    pub kind: DivergenceKind,
    pub reason: String,
    pub owner: String,
    pub expires: String,
    pub issue: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WaiverSet {
    pub waivers: Vec<Waiver>,
}

impl WaiverSet {
    #[must_use]
    pub fn new(waivers: Vec<Waiver>) -> Self {
        Self { waivers }
    }

    #[must_use]
    pub fn apply(&self, divergences: Vec<Divergence>) -> Vec<Divergence> {
        divergences
            .into_iter()
            .map(|mut divergence| {
                let waiver = self.waivers.iter().find(|waiver| {
                    waiver.case == divergence.case_id && waiver.kind == divergence.kind
                });

                if let Some(waiver) = waiver {
                    if is_expired(&waiver.expires) {
                        divergence.kind = DivergenceKind::WaiverExpired;
                        divergence.severity = Severity::Blocking;
                        divergence.message =
                            format!("Waiver expired on {}: {}", waiver.expires, waiver.reason);
                    } else {
                        divergence.severity = Severity::Allowed;
                        divergence.message = format!(
                            "Allowed by waiver until {}: {}",
                            waiver.expires, waiver.reason
                        );
                    }
                }

                divergence
            })
            .collect()
    }
}

fn is_expired(expires: &str) -> bool {
    let now = time::OffsetDateTime::now_utc().date();
    let today = format!(
        "{:04}-{:02}-{:02}",
        now.year(),
        u8::from(now.month()),
        now.day()
    );
    expires < today.as_str()
}

#[cfg(test)]
mod tests {
    use super::{Waiver, WaiverSet};
    use rewrit_model::{CaseId, Divergence, DivergenceKind, Severity};

    #[test]
    fn expired_waiver_blocks() {
        let waiver_set = WaiverSet::new(vec![Waiver {
            case: CaseId::new("billing.invoice.cancel.refund_event"),
            kind: DivergenceKind::SideEffectMismatch,
            reason: "not migrated".to_string(),
            owner: "billing-platform".to_string(),
            expires: "2000-01-01".to_string(),
            issue: None,
        }]);
        let divergences = waiver_set.apply(vec![Divergence {
            kind: DivergenceKind::SideEffectMismatch,
            severity: Severity::Blocking,
            case_id: CaseId::new("billing.invoice.cancel.refund_event"),
            suite: None,
            path: None,
            reference: None,
            candidate: None,
            message: "side effects differ".to_string(),
            machine_code: "side_effect_mismatch".to_string(),
            source_location: None,
            target_location: None,
            policy: None,
            normalizers_applied: Vec::new(),
            hint: None,
        }]);

        assert_eq!(divergences[0].kind, DivergenceKind::WaiverExpired);
        assert_eq!(divergences[0].severity, Severity::Blocking);
    }
}
