use std::collections::BTreeMap;

fn text(value: &str) -> rewrit::CanonicalValue {
    rewrit::CanonicalValue::String {
        value: value.to_string(),
    }
}

#[rewrit::case("billing.invoice.create.success")]
#[test]
fn creates_invoice() -> Result<(), Box<dyn std::error::Error>> {
    let mut headers = BTreeMap::new();
    headers.insert("content-type".to_string(), text("application/json"));
    headers.insert("x-request-id".to_string(), text("rust-request-id"));

    let mut fields = BTreeMap::new();
    fields.insert(
        "status".to_string(),
        rewrit::CanonicalValue::Integer {
            value: "201".to_string(),
        },
    );
    fields.insert(
        "headers".to_string(),
        rewrit::CanonicalValue::Object { fields: headers },
    );
    fields.insert(
        "body".to_string(),
        rewrit::CanonicalValue::Json {
            value: rewrit_django_to_rust_candidate::create_invoice(),
        },
    );

    rewrit::observe_canonical(
        Some(rewrit::CanonicalValue::Object { fields }),
        rewrit::CaseStatus::Passed,
        Vec::new(),
    )?;
    Ok(())
}
