use crate::app::common::CliError;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub struct NativeTaskData {
    pub task_id: String,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub status: String,
    pub raw_text: String,
}

pub fn ingest_from_json(path_str: &str) -> Result<Vec<NativeTaskData>, CliError> {
    let path = Path::new(path_str);
    if !path.exists() {
        return Err(CliError::not_found(format!(
            "JSON payload file not found: {}",
            path_str
        )));
    }
    let content = fs::read_to_string(path)?;
    let parsed: Value = serde_json::from_str(&content)
        .map_err(|e| CliError::argument(format!("invalid JSON payload: {}", e)))?;

    let mut tasks = Vec::new();

    if let Some(records) = parsed.get("records").and_then(|r| r.as_array()) {
        for record in records {
            let kind = record.get("kind").and_then(|k| k.as_str()).unwrap_or("");
            if kind == "orchestration.work_item.v1" || kind == "implementation_task" {
                let title = record
                    .get("title")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| CliError::argument("missing required field: title"))?
                    .to_string();
                let goal = record
                    .get("goal")
                    .and_then(|g| g.as_str())
                    .or_else(|| record.get("description").and_then(|d| d.as_str()))
                    .unwrap_or("")
                    .to_string();
                let priority = record
                    .get("priority")
                    .and_then(|p| p.as_str())
                    .unwrap_or("medium")
                    .to_string();
                let status = record
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("proposed")
                    .to_string();
                let task_id = record
                    .get("task_id")
                    .or_else(|| record.get("id"))
                    .and_then(|i| i.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                tasks.push(NativeTaskData {
                    task_id,
                    title,
                    description: goal,
                    priority,
                    status,
                    raw_text: record.to_string(),
                });
            }
        }
    } else {
        // Support direct single work_item payload
        let title = parsed
            .get("title")
            .and_then(|t| t.as_str())
            .ok_or_else(|| CliError::argument("missing required field: title"))?
            .to_string();
        let goal = parsed
            .get("goal")
            .and_then(|g| g.as_str())
            .or_else(|| parsed.get("description").and_then(|d| d.as_str()))
            .unwrap_or("")
            .to_string();
        let priority = parsed
            .get("priority")
            .and_then(|p| p.as_str())
            .unwrap_or("medium")
            .to_string();
        let status = parsed
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("proposed")
            .to_string();
        let task_id = parsed
            .get("task_id")
            .or_else(|| parsed.get("id"))
            .and_then(|i| i.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        tasks.push(NativeTaskData {
            task_id,
            title,
            description: goal,
            priority,
            status,
            raw_text: content,
        });
    }

    Ok(tasks)
}
