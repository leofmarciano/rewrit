use crate::compare::CompareContext;
use rewrit_model::{CanonicalValue, CaseId, Divergence, DivergenceKind, Severity};
use serde::Serialize;
use std::collections::BTreeSet;

pub fn divergence<T, U>(
    kind: DivergenceKind,
    case_id: CaseId,
    path: impl Into<String>,
    message: impl Into<String>,
    reference: Option<&T>,
    candidate: Option<&U>,
    ctx: &CompareContext,
) -> Divergence
where
    T: Serialize + ?Sized,
    U: Serialize + ?Sized,
{
    Divergence {
        machine_code: format!("{kind:?}").to_ascii_lowercase(),
        kind,
        severity: Severity::Blocking,
        case_id,
        suite: ctx.suite.clone(),
        path: Some(path.into()),
        reference: reference.and_then(|value| serde_json::to_value(value).ok()),
        candidate: candidate.and_then(|value| serde_json::to_value(value).ok()),
        message: message.into(),
        source_location: ctx.source_location.clone(),
        target_location: ctx.target_location.clone(),
        policy: Some(ctx.policy.name.clone()),
        normalizers_applied: ctx.normalizers_applied.clone(),
        hint: None,
    }
}

#[must_use]
pub fn value_type_name(value: &CanonicalValue) -> &'static str {
    value.kind_name()
}

pub fn value_divergences(
    case_id: &CaseId,
    reference: &CanonicalValue,
    candidate: &CanonicalValue,
    path: &str,
    ctx: &CompareContext,
) -> Vec<Divergence> {
    if path_is_ignored(path, ctx) || ctx.policy.values_equivalent(reference, candidate) {
        return Vec::new();
    }

    match (reference, candidate) {
        (CanonicalValue::Json { value: left }, CanonicalValue::Json { value: right }) => {
            json_divergences(case_id, left, right, path, ctx)
        }
        (CanonicalValue::Object { fields: left }, CanonicalValue::Object { fields: right }) => {
            let keys = left.keys().chain(right.keys()).collect::<BTreeSet<_>>();
            keys.into_iter()
                .flat_map(|key| {
                    let child_path = format!("{path}.{}", escape_path_segment(key));
                    if header_is_ignored(&child_path, key, ctx) {
                        return Vec::new();
                    }
                    match (left.get(key), right.get(key)) {
                        (Some(left), Some(right)) => {
                            value_divergences(case_id, left, right, &child_path, ctx)
                        }
                        (Some(left), None) => vec![divergence(
                            DivergenceKind::OutputMismatch,
                            case_id.clone(),
                            child_path,
                            "Candidate is missing a canonical object field.",
                            Some(left),
                            Some(&CanonicalValue::Absent),
                            ctx,
                        )],
                        (None, Some(right)) => vec![divergence(
                            DivergenceKind::OutputMismatch,
                            case_id.clone(),
                            child_path,
                            "Candidate has an extra canonical object field.",
                            Some(&CanonicalValue::Absent),
                            Some(right),
                            ctx,
                        )],
                        (None, None) => Vec::new(),
                    }
                })
                .collect()
        }
        (CanonicalValue::Array { items: left }, CanonicalValue::Array { items: right }) => {
            if path_is_unordered(path, ctx) {
                return unordered_array_divergences(case_id, left, right, path, ctx);
            }
            let len = left.len().max(right.len());
            (0..len)
                .flat_map(|idx| {
                    let child_path = format!("{path}[{idx}]");
                    match (left.get(idx), right.get(idx)) {
                        (Some(left), Some(right)) => {
                            value_divergences(case_id, left, right, &child_path, ctx)
                        }
                        (Some(left), None) => vec![divergence(
                            DivergenceKind::OutputMismatch,
                            case_id.clone(),
                            child_path,
                            "Candidate is missing an array item.",
                            Some(left),
                            Some(&CanonicalValue::Absent),
                            ctx,
                        )],
                        (None, Some(right)) => vec![divergence(
                            DivergenceKind::OutputMismatch,
                            case_id.clone(),
                            child_path,
                            "Candidate has an extra array item.",
                            Some(&CanonicalValue::Absent),
                            Some(right),
                            ctx,
                        )],
                        (None, None) => Vec::new(),
                    }
                })
                .collect()
        }
        _ if value_type_name(reference) != value_type_name(candidate) => vec![divergence(
            DivergenceKind::TypeMismatch,
            case_id.clone(),
            path.to_string(),
            format!(
                "Canonical value type differs: reference is {}, candidate is {}.",
                value_type_name(reference),
                value_type_name(candidate)
            ),
            Some(reference),
            Some(candidate),
            ctx,
        )],
        _ => vec![divergence(
            DivergenceKind::OutputMismatch,
            case_id.clone(),
            path.to_string(),
            "Canonical output values differ.",
            Some(reference),
            Some(candidate),
            ctx,
        )],
    }
}

fn json_divergences(
    case_id: &CaseId,
    reference: &serde_json::Value,
    candidate: &serde_json::Value,
    path: &str,
    ctx: &CompareContext,
) -> Vec<Divergence> {
    if path_is_ignored(path, ctx) || reference == candidate {
        return Vec::new();
    }

    match (reference, candidate) {
        (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
            let keys = left.keys().chain(right.keys()).collect::<BTreeSet<_>>();
            keys.into_iter()
                .flat_map(|key| {
                    let child_path = format!("{path}.{}", escape_path_segment(key));
                    match (left.get(key), right.get(key)) {
                        (Some(left), Some(right)) => {
                            json_divergences(case_id, left, right, &child_path, ctx)
                        }
                        (Some(left), None) => vec![divergence(
                            DivergenceKind::OutputMismatch,
                            case_id.clone(),
                            child_path,
                            "Candidate JSON object is missing a field.",
                            Some(left),
                            Some(&serde_json::Value::String("<ABSENT>".to_string())),
                            ctx,
                        )],
                        (None, Some(right)) => vec![divergence(
                            DivergenceKind::OutputMismatch,
                            case_id.clone(),
                            child_path,
                            "Candidate JSON object has an extra field.",
                            Some(&serde_json::Value::String("<ABSENT>".to_string())),
                            Some(right),
                            ctx,
                        )],
                        (None, None) => Vec::new(),
                    }
                })
                .collect()
        }
        (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
            if path_is_unordered(path, ctx) {
                return unordered_json_array_divergences(case_id, left, right, path, ctx);
            }
            let len = left.len().max(right.len());
            (0..len)
                .flat_map(|idx| {
                    let child_path = format!("{path}[{idx}]");
                    match (left.get(idx), right.get(idx)) {
                        (Some(left), Some(right)) => {
                            json_divergences(case_id, left, right, &child_path, ctx)
                        }
                        (Some(left), None) => vec![divergence(
                            DivergenceKind::OutputMismatch,
                            case_id.clone(),
                            child_path,
                            "Candidate JSON array is missing an item.",
                            Some(left),
                            Some(&serde_json::Value::String("<ABSENT>".to_string())),
                            ctx,
                        )],
                        (None, Some(right)) => vec![divergence(
                            DivergenceKind::OutputMismatch,
                            case_id.clone(),
                            child_path,
                            "Candidate JSON array has an extra item.",
                            Some(&serde_json::Value::String("<ABSENT>".to_string())),
                            Some(right),
                            ctx,
                        )],
                        (None, None) => Vec::new(),
                    }
                })
                .collect()
        }
        _ if json_kind(reference) != json_kind(candidate) => vec![divergence(
            DivergenceKind::TypeMismatch,
            case_id.clone(),
            path.to_string(),
            format!(
                "JSON value type differs: reference is {}, candidate is {}.",
                json_kind(reference),
                json_kind(candidate)
            ),
            Some(reference),
            Some(candidate),
            ctx,
        )],
        _ => vec![divergence(
            DivergenceKind::OutputMismatch,
            case_id.clone(),
            path.to_string(),
            "JSON values differ.",
            Some(reference),
            Some(candidate),
            ctx,
        )],
    }
}

fn json_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn header_is_ignored(path: &str, key: &str, ctx: &CompareContext) -> bool {
    path.starts_with("$.value.headers.")
        && ctx
            .policy
            .ignored_headers
            .iter()
            .any(|ignored| ignored.eq_ignore_ascii_case(key))
}

fn path_is_ignored(path: &str, ctx: &CompareContext) -> bool {
    ctx.policy
        .ignore_paths
        .iter()
        .any(|ignored| path_matches_policy(path, ignored))
}

fn path_is_unordered(path: &str, ctx: &CompareContext) -> bool {
    ctx.policy
        .unordered_paths
        .iter()
        .any(|unordered| path_matches_policy(path, unordered))
}

fn path_matches_policy(path: &str, ignored: &str) -> bool {
    candidate_policy_paths(path)
        .iter()
        .any(|path| path == ignored || wildcard_path_matches(path, ignored))
}

fn candidate_policy_paths(path: &str) -> Vec<String> {
    let mut paths = vec![path.to_string()];
    if let Some(suffix) = path.strip_prefix("$.value.body") {
        if suffix.is_empty() {
            paths.push("$".to_string());
        } else if let Some(suffix) = suffix.strip_prefix('.') {
            paths.push(format!("$.{suffix}"));
        } else if suffix.starts_with('[') {
            paths.push(format!("${suffix}"));
        }
    }
    if let Some(suffix) = path.strip_prefix("$.value") {
        if suffix.is_empty() {
            paths.push("$".to_string());
        } else if let Some(suffix) = suffix.strip_prefix('.') {
            paths.push(format!("$.{suffix}"));
        } else if suffix.starts_with('[') {
            paths.push(format!("${suffix}"));
        }
    }
    paths
}

fn wildcard_path_matches(path: &str, ignored: &str) -> bool {
    if !ignored.contains("[*]") {
        return false;
    }
    let prefix = ignored.split("[*]").next().unwrap_or_default();
    let suffix = ignored.split("[*]").nth(1).unwrap_or_default();
    path.starts_with(prefix) && path.ends_with(suffix)
}

fn unordered_json_array_divergences(
    case_id: &CaseId,
    reference: &[serde_json::Value],
    candidate: &[serde_json::Value],
    path: &str,
    ctx: &CompareContext,
) -> Vec<Divergence> {
    if sorted_json_items(reference) == sorted_json_items(candidate) {
        Vec::new()
    } else {
        vec![divergence(
            DivergenceKind::OutputMismatch,
            case_id.clone(),
            path.to_string(),
            "JSON array items differ after applying unordered array policy.",
            Some(reference),
            Some(candidate),
            ctx,
        )]
    }
}

fn unordered_array_divergences(
    case_id: &CaseId,
    reference: &[CanonicalValue],
    candidate: &[CanonicalValue],
    path: &str,
    ctx: &CompareContext,
) -> Vec<Divergence> {
    if sorted_serialized_items(reference) == sorted_serialized_items(candidate) {
        Vec::new()
    } else {
        vec![divergence(
            DivergenceKind::OutputMismatch,
            case_id.clone(),
            path.to_string(),
            "Canonical array items differ after applying unordered array policy.",
            Some(reference),
            Some(candidate),
            ctx,
        )]
    }
}

fn sorted_json_items(items: &[serde_json::Value]) -> Vec<String> {
    sorted_serialized_items(items)
}

fn sorted_serialized_items<T: Serialize>(items: &[T]) -> Vec<String> {
    let mut serialized = items
        .iter()
        .map(|item| serde_json::to_string(item).unwrap_or_default())
        .collect::<Vec<_>>();
    serialized.sort();
    serialized
}

fn escape_path_segment(segment: &str) -> String {
    if segment
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        segment.to_string()
    } else {
        format!("[{}]", serde_json::to_string(segment).unwrap_or_default())
    }
}
