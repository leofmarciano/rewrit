#[rewrit::case("sdk.macro.discovers_case")]
#[test]
fn case_macro_sets_current_case() -> Result<(), Box<dyn std::error::Error>> {
    let value = serde_json::json!({ "ok": true });
    rewrit::observe_json(&value)?;
    Ok(())
}
