use crate::app::common::CliError;
use crate::app::listing::{
    list_assignments_by_run, list_change_sets_by_run, list_failures, list_handoffs_by_run,
};
use earmark_store::CanonicalStore;
use serde_json::json;

pub(crate) fn build_run_graph<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<serde_json::Value, CliError> {
    let assignments = list_assignments_by_run(store, run_id)?;
    let change_sets = list_change_sets_by_run(store, run_id)?;
    let handoffs = list_handoffs_by_run(store, run_id)?;
    let failures = list_failures(store, Some(run_id), None)?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    nodes.push(json!({
        "id": run_id,
        "kind": "run",
        "label": format!("Run: {}", run_id)
    }));

    for a in assignments {
        nodes.push(json!({
            "id": a.id.as_str(),
            "kind": "assignment",
            "label": format!("Assignment: {}", a.transition_id)
        }));
        edges.push(json!({
            "from": run_id,
            "to": a.id.as_str(),
            "label": "created"
        }));

        if let Some(hid) = a.handoff_manifest_id {
            edges.push(json!({
                "from": a.id.as_str(),
                "to": hid.as_str(),
                "label": "emitted"
            }));
        }
    }

    for cs in change_sets {
        nodes.push(json!({
            "id": cs.id.as_str(),
            "kind": "change_set",
            "label": format!("Change Set: {}", cs.transition_id)
        }));
        if let Some(aid) = cs.assignment_id {
            edges.push(json!({
                "from": aid.as_str(),
                "to": cs.id.as_str(),
                "label": "produced"
            }));
        }
        if let Some(hid) = cs.handoff_manifest_id {
            edges.push(json!({
                "from": cs.id.as_str(),
                "to": hid.as_str(),
                "label": "linked_to"
            }));
        }
    }

    for ho in handoffs {
        nodes.push(json!({
            "id": ho.id.as_str(),
            "kind": "handoff",
            "label": format!("Handoff: {}", ho.from_transition_id)
        }));
    }

    for f in failures {
        let fid = f["failure_id"].as_str().unwrap_or("");
        nodes.push(json!({
            "id": fid,
            "kind": "failure",
            "label": format!("Failure: {}", f["error_type"].as_str().unwrap_or(""))
        }));
        if let Some(aid) = f["assignment_id"].as_str() {
            edges.push(json!({
                "from": aid,
                "to": fid,
                "label": "failed_at"
            }));
        }
    }

    Ok(json!({
        "nodes": nodes,
        "edges": edges
    }))
}
