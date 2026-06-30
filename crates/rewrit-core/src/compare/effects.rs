use crate::policy::Policy;
use rewrit_model::{DbDelta, Effect};
use std::collections::BTreeMap;

#[must_use]
pub fn effects_equivalent(reference: &[Effect], candidate: &[Effect], policy: &Policy) -> bool {
    normalize_effects(reference, policy, RuntimeSide::Reference)
        == normalize_effects(candidate, policy, RuntimeSide::Candidate)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeSide {
    Reference,
    Candidate,
}

fn normalize_effects(effects: &[Effect], policy: &Policy, side: RuntimeSide) -> Vec<Effect> {
    effects
        .iter()
        .cloned()
        .map(|effect| match effect {
            Effect::DbDelta(delta) => Effect::DbDelta(normalize_db_delta(delta, policy, side)),
            effect => effect,
        })
        .collect()
}

fn normalize_db_delta(delta: DbDelta, policy: &Policy, side: RuntimeSide) -> DbDelta {
    if side == RuntimeSide::Reference {
        return delta;
    }

    let Some((reference_table, db_map)) = policy
        .db_maps
        .iter()
        .find(|(_, db_map)| db_map.target_table == delta.table)
    else {
        return delta;
    };

    let reverse_fields = db_map
        .fields
        .iter()
        .map(|(reference, candidate)| (candidate.clone(), reference.clone()))
        .collect::<BTreeMap<_, _>>();

    DbDelta {
        connection: delta.connection,
        table: reference_table.clone(),
        inserted: delta
            .inserted
            .into_iter()
            .map(|row| normalize_row(row, &reverse_fields))
            .collect(),
        updated: delta
            .updated
            .into_iter()
            .map(|row| normalize_row(row, &reverse_fields))
            .collect(),
        deleted: delta
            .deleted
            .into_iter()
            .map(|row| normalize_row(row, &reverse_fields))
            .collect(),
    }
}

fn normalize_row(
    row: BTreeMap<String, rewrit_model::CanonicalValue>,
    reverse_fields: &BTreeMap<String, String>,
) -> BTreeMap<String, rewrit_model::CanonicalValue> {
    row.into_iter()
        .map(|(field, value)| (reverse_fields.get(&field).cloned().unwrap_or(field), value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rewrit_model::{
        CacheOperation, CanonicalValue, DbMap, EmailEmission, EventEmission, FileDelta,
        FileOperation, LogRecord, QueueMessage,
    };

    #[test]
    fn maps_candidate_db_delta_table_and_fields_to_reference_shape() {
        let reference = vec![Effect::DbDelta(DbDelta {
            connection: "default".to_string(),
            table: "invoices".to_string(),
            inserted: vec![BTreeMap::from([
                ("id".to_string(), text("inv_123")),
                ("amount".to_string(), text("199.90")),
            ])],
            updated: Vec::new(),
            deleted: Vec::new(),
        })];
        let candidate = vec![Effect::DbDelta(DbDelta {
            connection: "default".to_string(),
            table: "billing_invoices".to_string(),
            inserted: vec![BTreeMap::from([
                ("invoice_id".to_string(), text("inv_123")),
                ("total_amount".to_string(), text("199.90")),
            ])],
            updated: Vec::new(),
            deleted: Vec::new(),
        })];
        let policy = Policy {
            db_maps: BTreeMap::from([(
                "invoices".to_string(),
                DbMap {
                    target_table: "billing_invoices".to_string(),
                    fields: BTreeMap::from([
                        ("id".to_string(), "invoice_id".to_string()),
                        ("amount".to_string(), "total_amount".to_string()),
                    ]),
                },
            )]),
            ..Policy::default()
        };

        assert!(effects_equivalent(&reference, &candidate, &policy));
    }

    #[test]
    fn compares_non_db_effect_variants_by_canonical_equality() {
        let reference = vec![
            Effect::FileDelta(FileDelta {
                path: "out/report.json".to_string(),
                operation: FileOperation::Created,
                sha256: Some("abc".to_string()),
            }),
            Effect::QueueMessage(QueueMessage {
                queue: "billing".to_string(),
                topic: Some("invoice.created".to_string()),
                payload: text("inv_123"),
            }),
            Effect::Event(EventEmission {
                name: "InvoiceCreated".to_string(),
                payload: text("inv_123"),
            }),
            Effect::Email(EmailEmission {
                to: vec!["customer@example.com".to_string()],
                subject: "Invoice".to_string(),
                body: Some("created".to_string()),
            }),
            Effect::CacheOperation(CacheOperation {
                operation: "set".to_string(),
                key: "invoice:inv_123".to_string(),
                value: Some(text("open")),
            }),
            Effect::Log(LogRecord {
                level: "info".to_string(),
                message: "invoice created".to_string(),
                fields: BTreeMap::from([("invoice_id".to_string(), "inv_123".to_string())]),
            }),
        ];
        let mut candidate = reference.clone();

        assert!(effects_equivalent(
            &reference,
            &candidate,
            &Policy::default()
        ));

        candidate[1] = Effect::QueueMessage(QueueMessage {
            queue: "billing".to_string(),
            topic: Some("invoice.created".to_string()),
            payload: text("inv_456"),
        });

        assert!(!effects_equivalent(
            &reference,
            &candidate,
            &Policy::default()
        ));
    }

    fn text(value: &str) -> CanonicalValue {
        CanonicalValue::String {
            value: value.to_string(),
        }
    }
}
