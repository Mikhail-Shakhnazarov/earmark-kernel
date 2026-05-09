use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

#[derive(Default)]
struct CommandMetric {
    calls: u64,
    failures: u64,
    total_duration_ms: u128,
}

#[derive(Default)]
struct MetricsState {
    by_command: BTreeMap<&'static str, CommandMetric>,
}

fn state() -> &'static Mutex<MetricsState> {
    static STATE: OnceLock<Mutex<MetricsState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(MetricsState::default()))
}

pub fn record_command_result(name: &'static str, ok: bool, duration: Duration) {
    if let Ok(mut guard) = state().lock() {
        let entry = guard.by_command.entry(name).or_default();
        entry.calls += 1;
        if !ok {
            entry.failures += 1;
        }
        entry.total_duration_ms += duration.as_millis();
    }
    tracing::info!(
        command = name,
        ok,
        duration_ms = duration.as_millis() as u64,
        "command execution"
    );
}

pub fn snapshot() -> serde_json::Value {
    let Ok(guard) = state().lock() else {
        return serde_json::json!({"metrics": "unavailable"});
    };
    let commands = guard
        .by_command
        .iter()
        .map(|(name, metric)| {
            let avg_ms = if metric.calls == 0 {
                0.0
            } else {
                metric.total_duration_ms as f64 / metric.calls as f64
            };
            serde_json::json!({
                "command": name,
                "calls": metric.calls,
                "failures": metric.failures,
                "avg_duration_ms": avg_ms,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({"commands": commands})
}
