use crate::policy::Policy;
use rewrit_model::CanonicalError;

#[must_use]
pub fn errors_equivalent(
    reference: Option<&CanonicalError>,
    candidate: Option<&CanonicalError>,
    policy: &Policy,
) -> bool {
    match (reference, candidate) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.kind == right.kind
                && left.code == right.code
                && left.http_status == right.http_status
                && (policy.ignore_stack_trace || left.frames == right.frames)
                && (left.normalized_message == right.normalized_message
                    || left.message == right.message
                    || policy.ignore_error_message)
        }
        _ => false,
    }
}

