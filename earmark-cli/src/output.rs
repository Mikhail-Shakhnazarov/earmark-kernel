use serde_json::json;

pub const CONTRACT_VERSION: &str = "0.2.0";

pub fn emit_json_envelope(value: serde_json::Value) {
    let envelope = json!({
        "contract_version": CONTRACT_VERSION,
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
    // For machine readability, even errors in JSON mode go to stdout
    // if the user requested --json for orchestration.
    // However, some prefer stderr for errors.
    // Earmark CLI historically used stdout for the JSON envelope even for errors
    // to keep the stream parseable.
    eprintln!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
    );
}

