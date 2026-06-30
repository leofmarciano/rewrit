pub fn create_invoice() -> serde_json::Value {
    serde_json::json!({
        "id": "inv_123",
        "amount": "199.90",
        "currency": "BRL",
        "status": "open"
    })
}
