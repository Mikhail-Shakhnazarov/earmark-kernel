use std::fs;

use chrono::Utc;
use earmark_core::{Kind, ObjectId};
use earmark_store::{CanonicalStore, StoredObject};
use serde_json::json;

use crate::cli::ReportAction;
use crate::app::{emit, CliError};

pub fn handle<S: CanonicalStore>(
    store: &S,
    as_json: bool,
    action: ReportAction,
) -> Result<(), CliError> {
    match action {
        ReportAction::Trial { trial_id } => handle_trial_report(store, as_json, trial_id),
        _ => Err(CliError::argument("report action not yet implemented on this surface")),
    }
}

fn handle_trial_report<S: CanonicalStore>(
    store: &S,
    as_json: bool,
    trial_id_arg: Option<String>,
) -> Result<(), CliError> {
    // 1. Resolve Trial ID
    let trial_id = if let Some(id_str) = trial_id_arg {
        if id_str == "latest" {
            find_latest_trial(store)?
        } else {
            ObjectId::parse(id_str).map_err(|e| CliError::argument(e.to_string()))?
        }
    } else {
        // Look for active trial in .earmark/derived/active_trial.json
        load_active_trial_id(store)?
    };

    let trial_obj = store.read_head(&trial_id)?.ok_or_else(|| CliError::not_found(format!("trial {}", trial_id)))?;
    let trial_payload: serde_json::Value = serde_json::from_slice(&trial_obj.payload.bytes)?;

    let title = trial_payload["title"].as_str().unwrap_or("Untitled Trial");
    let goal = trial_payload["goal"].as_str().unwrap_or("No goal specified");

    // 2. Find Frictions
    let all_objects = store.scan_objects()?;
    let mut frictions = Vec::new();
    
    // Find relations: friction_belongs_to_trial where target is trial_id
    for obj in &all_objects {
        if obj.envelope.kind == Kind::Relation {
            let rel: earmark_core::RelationPayload = serde_json::from_slice(&obj.payload.bytes)?;
            if rel.relation_type == "friction_belongs_to_trial" && rel.target.id == trial_id {
                // Source is a friction
                if let Some(friction_obj) = store.read_head(&rel.source.id)? {
                    frictions.push(friction_obj);
                }
            }
        }
    }

    // 3. Aggregate Data
    let mut report_frictions = Vec::new();
    let mut resolved_count = 0;

    for friction in frictions {
        let f_id = friction.envelope.id.clone();
        let f_payload: serde_json::Value = serde_json::from_slice(&friction.payload.bytes)?;
        
        let mut repairs = Vec::new();
        let mut verifications = Vec::new();
        let mut is_resolved = false;

        for obj in &all_objects {
            if obj.envelope.kind == Kind::Relation {
                let rel: earmark_core::RelationPayload = serde_json::from_slice(&obj.payload.bytes)?;
                if rel.target.id == f_id {
                    if rel.relation_type == "repair_addresses_friction" {
                        if let Some(repair) = store.read_head(&rel.source.id)? {
                            repairs.push(repair);
                        }
                    } else if rel.relation_type == "verification_checks_friction" {
                        if let Some(verification) = store.read_head(&rel.source.id)? {
                            let v_payload: serde_json::Value = serde_json::from_slice(&verification.payload.bytes)?;
                            if v_payload["result"].as_str() == Some("passed") {
                                is_resolved = true;
                            }
                            verifications.push(verification);
                        }
                    }
                }
            }
        }

        if is_resolved {
            resolved_count += 1;
        }

        report_frictions.push(json!({
            "id": f_id.as_str(),
            "summary": f_payload["summary"],
            "severity": f_payload["severity"],
            "type": f_payload["type"],
            "is_resolved": is_resolved,
            "repairs": repairs.iter().map(|r| r.envelope.id.as_str()).collect::<Vec<_>>(),
            "verifications": verifications.iter().map(|v| v.envelope.id.as_str()).collect::<Vec<_>>(),
        }));
    }

    let metrics = json!({
        "total_frictions": report_frictions.len(),
        "resolved_count": resolved_count,
        "outstanding_count": report_frictions.len() - resolved_count,
        "resolution_rate": if report_frictions.is_empty() { 0.0 } else { (resolved_count as f64 / report_frictions.len() as f64) * 100.0 }
    });

    // 4. Generate Markdown
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let report_filename = format!("trial-report-{}-{}.md", trial_id.as_str(), timestamp);
    let report_path = store.root().join("docs").join("trials").join("reports").join(&report_filename);
    
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut md = format!("# Trial Report: {}\n\n", title);
    md.push_str(&format!("**Trial ID**: `{}`  \n", trial_id));
    md.push_str(&format!("**Goal**: {}  \n", goal));
    md.push_str(&format!("**Generated**: {}  \n\n", Utc::now().to_rfc3339()));

    md.push_str("## Summary Metrics\n\n");
    md.push_str("| Metric | Value |\n");
    md.push_str("| :--- | :--- |\n");
    md.push_str(&format!("| Total Frictions | {} |\n", metrics["total_frictions"]));
    md.push_str(&format!("| Resolved | {} |\n", metrics["resolved_count"]));
    md.push_str(&format!("| Outstanding | {} |\n", metrics["outstanding_count"]));
    md.push_str(&format!("| Resolution Rate | {:.1}% |\n\n", metrics["resolution_rate"].as_f64().unwrap_or(0.0)));

    md.push_str("## Friction Details\n\n");
    for f in &report_frictions {
        let status = if f["is_resolved"].as_bool().unwrap_or(false) { "✅ Resolved" } else { "❌ Outstanding" };
        md.push_str(&format!("### {} [{}]\n", f["summary"].as_str().unwrap_or("Unknown"), status));
        md.push_str(&format!("- **ID**: `{}`\n", f["id"].as_str().unwrap_or("-")));
        md.push_str(&format!("- **Severity**: {}\n", f["severity"].as_str().unwrap_or("-")));
        md.push_str(&format!("- **Type**: {}\n", f["type"].as_str().unwrap_or("-")));
        
        let repairs = f["repairs"].as_array().unwrap();
        if !repairs.is_empty() {
            md.push_str("- **Repairs**: ");
            md.push_str(&repairs.iter().map(|r| format!("`{}`", r.as_str().unwrap())).collect::<Vec<_>>().join(", "));
            md.push_str("\n");
        }

        let verifications = f["verifications"].as_array().unwrap();
        if !verifications.is_empty() {
            md.push_str("- **Verifications**: ");
            md.push_str(&verifications.iter().map(|v| format!("`{}`", v.as_str().unwrap())).collect::<Vec<_>>().join(", "));
            md.push_str("\n");
        }
        md.push_str("\n");
    }

    fs::write(&report_path, &md)?;

    emit(as_json, json!({
        "ok": true,
        "summary": format!("Report generated for trial {}", title),
        "report_path": report_path.display().to_string(),
        "metrics": metrics,
    }));

    Ok(())
}

fn find_latest_trial<S: CanonicalStore>(store: &S) -> Result<ObjectId, CliError> {
    let objects = store.scan_objects()?;
    let mut trials = Vec::new();
    for obj in objects {
        if obj.envelope.class.as_deref() == Some("trial") {
            trials.push(obj);
        }
    }
    trials.sort_by_key(|obj| obj.envelope.created_at);
    trials.last()
        .map(|obj| obj.envelope.id.clone())
        .ok_or_else(|| CliError::not_found("no trials found in store"))
}

fn load_active_trial_id<S: CanonicalStore>(store: &S) -> Result<ObjectId, CliError> {
    let path = store.root().join(".earmark").join("derived").join("active_trial.json");
    if !path.exists() {
        return Err(CliError::not_found("no active trial found; provide a trial ID or run em trial start"));
    }
    let content = fs::read_to_string(path)?;
    let val: serde_json::Value = serde_json::from_str(&content)?;
    let id_str = val["trial_id"].as_str().ok_or_else(|| CliError::argument("invalid active_trial.json"))?;
    ObjectId::parse(id_str).map_err(|e| CliError::argument(e.to_string()))
}
