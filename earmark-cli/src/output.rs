use serde_json::json;
use std::cell::RefCell;

pub const CLI_CONTRACT_VERSION: &str = "0.3.0";

thread_local! {
    static CLI_CTX: RefCell<Option<CliContext>> = const { RefCell::new(None) };
}

#[allow(dead_code)]
pub struct CliContext {
    pub command_name: &'static str,
    pub as_json: bool,
}

pub fn init_context(ctx: CliContext) {
    CLI_CTX.with(|cell| {
        *cell.borrow_mut() = Some(ctx);
    });
}

#[allow(dead_code)]
pub fn with_context<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&CliContext) -> R,
{
    CLI_CTX.with(|cell| cell.borrow().as_ref().map(f))
}

#[allow(dead_code)]
pub fn as_json_mode() -> bool {
    with_context(|ctx| ctx.as_json).unwrap_or(false)
}

#[allow(dead_code)]
pub fn command_name() -> Option<&'static str> {
    with_context(|ctx| ctx.command_name)
}

pub fn emit_json_envelope(value: serde_json::Value) {
    let envelope = json!({
        "contract_version": CLI_CONTRACT_VERSION,
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
        "contract_version": CLI_CONTRACT_VERSION,
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

pub fn emit_error_envelope_with_kind(message: &str, kind: &str) {
    let value = json!({
        "contract_version": CLI_CONTRACT_VERSION,
        "ok": false,
        "error": {
            "message": message,
            "code": kind,
        }
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
    );
}
