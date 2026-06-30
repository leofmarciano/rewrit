use rewrit_model::CanonicalValue;

#[must_use]
pub fn canonical_json_equivalent(reference: &CanonicalValue, candidate: &CanonicalValue) -> bool {
    reference == candidate
}
