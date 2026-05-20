use crate::app::common::CliError;
use std::process::Command;

pub struct EngramTaskData {
    pub task_id: String,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub status: String,
    pub raw_text: String,
}

pub fn ingest_from_engram(task_id: &str) -> Result<EngramTaskData, CliError> {
    let engram_bin = std::env::var("ENGRAM_BIN").unwrap_or_else(|_| "engram".to_string());

    let output = Command::new(&engram_bin)
        .args(["task", "show", task_id])
        .output();

    let output = match output {
        Ok(out) => out,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(CliError::argument(
                    "Engram executable not found. Set ENGRAM_BIN or install `engram` on PATH.",
                ));
            }
            return Err(CliError::argument(format!(
                "Failed to execute engram: {}",
                e
            )));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr_trimmed = stderr.trim();
        if stderr_trimmed.contains("Not found") || stderr_trimmed.contains("not found") {
            return Err(CliError::not_found(format!("task {} not found", task_id)));
        }
        return Err(CliError::argument(format!(
            "engram command failed: {}",
            stderr_trimmed
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let fields = parse_engram_fields(&stdout);

    let resolved_task_id = fields
        .get("ID")
        .cloned()
        .unwrap_or_else(|| task_id.to_string());
    let title = fields.get("Title").cloned().unwrap_or_default();
    if title.is_empty() {
        return Err(CliError::argument(
            "engram output did not include a task title",
        ));
    }
    let description = fields.get("Description").cloned().unwrap_or_default();
    let priority = fields.get("Priority").cloned().unwrap_or_default();
    let status_raw = fields.get("Status").cloned().unwrap_or_default();
    let status = map_engram_status(&status_raw).to_string();

    Ok(EngramTaskData {
        task_id: resolved_task_id,
        title,
        description,
        priority,
        status,
        raw_text: stdout,
    })
}

fn parse_engram_fields(text: &str) -> std::collections::HashMap<String, String> {
    let mut fields = std::collections::HashMap::new();
    for line in text.lines() {
        if !line.starts_with("  ") {
            continue;
        }
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find(':') {
            let key = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 1..].trim().to_string();
            if !key.is_empty()
                && !value.is_empty()
                && key.chars().all(|c| c.is_alphanumeric() || c == ' ')
            {
                fields.insert(key, value);
            }
        }
    }
    fields
}

fn map_engram_status(status: &str) -> &'static str {
    match status {
        "Todo" => "proposed",
        "InProgress" => "dispatched",
        "Done" => "implemented",
        "Blocked" => "proposed",
        "Cancelled" => "closed",
        _ => "proposed",
    }
}
