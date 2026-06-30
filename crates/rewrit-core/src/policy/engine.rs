use crate::compare::{Comparator, CompareContext, Comparison, StrictComparator};
use crate::normalize::{NormalizationPipeline, NormalizationResult, NormalizeContext};
use crate::policy::WaiverSet;
use rewrit_model::{CanonicalValue, Observation};

#[derive(Debug, Clone)]
pub struct Policy {
    pub name: String,
    pub compare_stdout: bool,
    pub compare_stderr: bool,
    pub compare_exit_code: bool,
    pub compare_duration: bool,
    pub allow_null_absent_equivalence: bool,
    pub allow_integer_float_equivalence: bool,
    pub allow_header_case_difference: bool,
    pub allow_object_key_order_difference: bool,
    pub fail_on_orphan_candidate: bool,
    pub decimal_as_string: bool,
    pub ignore_stack_trace: bool,
    pub ignore_error_message: bool,
    pub ignore_paths: Vec<String>,
    pub ignored_headers: Vec<String>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            name: "strict".to_string(),
            compare_stdout: false,
            compare_stderr: false,
            compare_exit_code: true,
            compare_duration: false,
            allow_null_absent_equivalence: false,
            allow_integer_float_equivalence: false,
            allow_header_case_difference: true,
            allow_object_key_order_difference: true,
            fail_on_orphan_candidate: false,
            decimal_as_string: true,
            ignore_stack_trace: true,
            ignore_error_message: false,
            ignore_paths: Vec::new(),
            ignored_headers: vec![
                "date".to_string(),
                "server".to_string(),
                "x-request-id".to_string(),
            ],
        }
    }
}

impl Policy {
    #[must_use]
    pub fn values_equivalent(
        &self,
        reference: &CanonicalValue,
        candidate: &CanonicalValue,
    ) -> bool {
        if reference == candidate {
            return true;
        }

        if self.allow_null_absent_equivalence
            && matches!(
                (reference, candidate),
                (CanonicalValue::Null, CanonicalValue::Absent)
                    | (CanonicalValue::Absent, CanonicalValue::Null)
            )
        {
            return true;
        }

        if self.allow_integer_float_equivalence {
            if let (Some(left), Some(right)) = (numeric_text(reference), numeric_text(candidate)) {
                return left.parse::<f64>().ok() == right.parse::<f64>().ok();
            }
        }

        false
    }
}

fn numeric_text(value: &CanonicalValue) -> Option<&str> {
    match value {
        CanonicalValue::Integer { value }
        | CanonicalValue::Decimal { value }
        | CanonicalValue::Float { value } => Some(value.as_str()),
        _ => None,
    }
}

pub struct PolicyEngine {
    pub normalizers: NormalizationPipeline,
    pub comparator: Box<dyn Comparator>,
    pub waivers: WaiverSet,
    pub policy: Policy,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self {
            normalizers: NormalizationPipeline::default(),
            comparator: Box::new(StrictComparator),
            waivers: WaiverSet::default(),
            policy: Policy::default(),
        }
    }
}

impl PolicyEngine {
    pub fn normalize(
        &self,
        observation: Observation,
        ctx: &NormalizeContext,
    ) -> Result<NormalizationResult, crate::normalize::NormalizeError> {
        self.normalizers.normalize(observation, ctx)
    }

    #[must_use]
    pub fn compare(
        &self,
        reference: &Observation,
        candidate: &Observation,
        ctx: CompareContext,
    ) -> Comparison {
        let mut comparison = self.comparator.compare(reference, candidate, &ctx);
        comparison.divergences = self.waivers.apply(comparison.divergences);
        comparison.equivalent = comparison
            .divergences
            .iter()
            .all(|divergence| !matches!(divergence.severity, rewrit_model::Severity::Blocking));
        comparison
    }
}

#[cfg(test)]
mod tests {
    use super::Policy;
    use rewrit_model::CanonicalValue;

    #[test]
    fn treats_null_and_absent_as_different_by_default() {
        let policy = Policy::default();
        assert!(!policy.values_equivalent(&CanonicalValue::Null, &CanonicalValue::Absent));
    }

    #[test]
    fn allows_null_absent_only_when_policy_says_so() {
        let policy = Policy {
            allow_null_absent_equivalence: true,
            ..Policy::default()
        };
        assert!(policy.values_equivalent(&CanonicalValue::Null, &CanonicalValue::Absent));
    }

    #[test]
    fn detects_decimal_string_vs_float_mismatch_by_default() {
        let policy = Policy::default();
        assert!(!policy.values_equivalent(
            &CanonicalValue::Decimal {
                value: "199.90".to_string()
            },
            &CanonicalValue::Float {
                value: "199.9".to_string()
            }
        ));
    }
}
