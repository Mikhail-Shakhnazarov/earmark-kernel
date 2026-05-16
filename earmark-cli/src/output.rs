use serde_json::json;

pub const CONTRACT_VERSION: &str = "0.2.0";

pub fn emit_json_envelope(value: serde_json::Value) {
    let envelope = json!({
        "contract_version": CONTRACT_VERSION,
        "ok": true,
        "data": value
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "{}".to_string())
    );
}

pub fn emit_error_envelope(message: &str) {
    let value = json!({
        "contract_version": CONTRACT_VERSION,
        "ok": false,
        "error": {
            "message": message,
        }
    });
    // For machine readability, all JSON envelopes (including errors) are emitted to stdout.
    // This ensures that an orchestration pipeline can always parse the output as JSON
    // without needing to redirect stderr.
    println!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
    );
}
