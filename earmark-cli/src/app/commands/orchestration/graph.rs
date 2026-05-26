use crate::app::commands::orchestration::common::*;
use crate::app::common::CliError;
use earmark_core::ObjectId;
use earmark_index::DerivedIndex;
use earmark_store::GitCanonicalStore;
use std::collections::{HashSet, VecDeque};

pub struct OrchestrationGraphNode {
    pub object_id: ObjectId,
    pub class: String,
    pub payload: serde_json::Value,
    pub timestamp: i64,
}

pub fn traverse_orchestration_graph(
    index: &DerivedIndex,
    store: &GitCanonicalStore,
    start_id: &ObjectId,
) -> Result<Vec<OrchestrationGraphNode>, CliError> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut nodes = Vec::new();

    let orch_classes: HashSet<&str> = [
        "work_item",
        "dispatch",
        "evidence",
        "git_snapshot",
        "gate_result",
        "review",
        "closure",
        "context_packet",
        "followup_task",
        "trace_event",
    ]
    .into_iter()
    .collect();

    let start_id_str = start_id.as_str().to_string();
    queue.push_back(start_id_str.clone());
    visited.insert(start_id_str.clone());

    while let Some(current_id_str) = queue.pop_front() {
        let (oid, class, summary, payload) =
            match find_orchestration_task(index, store, &current_id_str) {
                Ok(Some(data)) => data,
                _ => continue,
            };

        if !orch_classes.contains(class.as_str()) && current_id_str != start_id.as_str() {
            continue;
        }

        let timestamp = chrono::DateTime::parse_from_rfc3339(&summary.created_at)
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(0);

        nodes.push(OrchestrationGraphNode {
            object_id: oid.clone(),
            class: class.clone(),
            payload,
            timestamp,
        });

        let relations = index.relation_adjacency(&oid, false)?;
        for rel in relations {
            let neighbor = if rel.source_object_id == current_id_str {
                rel.target_object_id
            } else {
                rel.source_object_id
            };

            if !visited.contains(&neighbor) {
                visited.insert(neighbor.clone());
                queue.push_back(neighbor);
            }
        }
    }

    Ok(nodes)
}
