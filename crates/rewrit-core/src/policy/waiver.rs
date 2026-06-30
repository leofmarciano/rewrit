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
                        divergence.message = format!(
                            "Waiver expired on {}: {}",
                            waiver.expires, waiver.reason
                        );
                    } else {
                        divergence.severity = Severity::Allowed;
                        divergence.message =
                            format!("Allowed by waiver until {}: {}", waiver.expires, waiver.reason);
                    }
                }

                divergence
            })
            .collect()
    }
}

fn is_expired(expires: &str) -> bool {
    let now = time::OffsetDateTime::now_utc().date();
    let today = format!("{:04}-{:02}-{:02}", now.year(), u8::from(now.month()), now.day());
    expires < today.as_str()
}

