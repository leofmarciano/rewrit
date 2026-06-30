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
    use rewrit_model::{CanonicalValue, DbMap};

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

    fn text(value: &str) -> CanonicalValue {
        CanonicalValue::String {
            value: value.to_string(),
        }
    }
}
