use proptest::prelude::*;
use rewrit_core::compare::Comparator;
use rewrit_core::normalize::regex::RegexNormalizer;
use rewrit_core::{
    CompareContext, NormalizationPipeline, NormalizeContext, Policy, StrictComparator,
};
use rewrit_model::{CanonicalValue, CapturedText, CaseId, CaseStatus, Observation, RuntimeId};
use std::collections::BTreeMap;

proptest! {
    #[test]
    fn regex_normalization_is_idempotent(value in canonical_value_strategy()) {
        let pipeline = NormalizationPipeline::new(vec![Box::new(
            RegexNormalizer::new("digits", r"\d+", "<N>").expect("regex")
        )]);
        let once = pipeline
            .normalize(observation(value), &NormalizeContext::default())
            .expect("first normalize")
            .observation;
        let twice = pipeline
            .normalize(once.clone(), &NormalizeContext::default())
            .expect("second normalize")
            .observation;

        prop_assert_eq!(once, twice);
    }

    #[test]
    fn strict_compare_observation_to_itself_is_equivalent(value in canonical_value_strategy()) {
        let observation = observation(value);
        let comparison = StrictComparator.compare(&observation, &observation, &compare_context());

        prop_assert!(comparison.equivalent);
        prop_assert!(comparison.divergences.is_empty());
    }
}

fn canonical_value_strategy() -> impl Strategy<Value = CanonicalValue> {
    let leaf = prop_oneof![
        Just(CanonicalValue::Null),
        Just(CanonicalValue::Absent),
        any::<bool>().prop_map(|value| CanonicalValue::Bool { value }),
        "-?[0-9]{1,12}".prop_map(|value| CanonicalValue::Integer { value }),
        "[0-9]{1,8}\\.[0-9]{2}".prop_map(|value| CanonicalValue::Decimal { value }),
        "[a-zA-Z0-9_ ./:-]{0,32}".prop_map(|value| CanonicalValue::String { value }),
    ];

    leaf.prop_recursive(3, 32, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4)
                .prop_map(|items| CanonicalValue::Array { items }),
            prop::collection::btree_map("[a-z_]{1,8}", inner, 0..4)
                .prop_map(|fields| CanonicalValue::Object { fields }),
        ]
    })
}

fn observation(value: CanonicalValue) -> Observation {
    Observation {
        case_id: CaseId::new("property.case"),
        runtime_id: RuntimeId::new("runtime"),
        status: CaseStatus::Passed,
        value: Some(value),
        error: None,
        stdout: CapturedText::default(),
        stderr: CapturedText::default(),
        exit_code: Some(0),
        duration_ms: 1,
        effects: Vec::new(),
        artifacts: Vec::new(),
        metadata: BTreeMap::new(),
    }
}

fn compare_context() -> CompareContext {
    CompareContext {
        policy: Policy::default(),
        suite: None,
        source_location: None,
        target_location: None,
        normalizers_applied: Vec::new(),
    }
}
