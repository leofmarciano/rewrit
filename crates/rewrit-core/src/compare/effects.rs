use rewrit_model::Effect;

#[must_use]
pub fn effects_equivalent(reference: &[Effect], candidate: &[Effect]) -> bool {
    reference == candidate
}

