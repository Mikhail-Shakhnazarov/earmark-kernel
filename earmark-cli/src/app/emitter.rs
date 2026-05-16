pub(crate) fn emit(as_json: bool, value: serde_json::Value) {
    if as_json {
        crate::output::emit_json_envelope(value);
    } else {
        match render_explanation(&value) {
            Some(explanation) => println!("{}", explanation),
            None => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
        }
    }
}

fn render_explanation(value: &serde_json::Value) -> Option<String> {
    let kind = value.get("kind")?.as_str()?;
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let summary = value.get("summary")?.as_str().unwrap_or("");
    let next_commands = value.get("next_commands").and_then(|v| v.as_array());

    let mut output = String::new();
    output.push_str(&format!(
        "--- {} Explanation: {} ---\n\n",
        kind.to_uppercase(),
        id
    ));
    output.push_str(&format!("Summary: {}\n\n", summary));

    match kind {
        "query_results" => {
            let results = value.get("results")?.as_array()?;
            output.push_str("Matches:\n");
            for obj in results {
                let object_id = obj.get("object_id")?.as_str()?;
                let class = obj.get("class").and_then(|v| v.as_str()).unwrap_or("none");
                let title = obj
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("no title");
                output.push_str(&format!("- [{}] {} (class: {})\n", object_id, title, class));
                if let Some(headers) = obj.get("headers").and_then(|v| v.as_object()) {
                    if !headers.is_empty() {
                        let h_strs: Vec<String> = headers
                            .iter()
                            .map(|(k, v)| format!("{}={}", k, v.as_str().unwrap_or("")))
                            .collect();
                        output.push_str(&format!("  Headers: {}\n", h_strs.join(", ")));
                    }
                }
                if let Some(standing) = obj.get("standing").and_then(|v| v.as_object()) {
                    if !standing.is_empty() {
                        let s_strs: Vec<String> = standing
                            .iter()
                            .map(|(k, v)| format!("{}:{}", k, v.as_str().unwrap_or("")))
                            .collect();
                        output.push_str(&format!("  Standing: {}\n", s_strs.join(", ")));
                    }
                }
            }
        }
        "status" => {
            output.push_str("Workspace Overview:\n");
            output.push_str(&format!(
                "  Objects: {}\n",
                value.get("object_count")?.as_u64()?
            ));
            output.push_str(&format!(
                "  Active Systems: {}\n",
                value.get("active_system_count")?.as_u64()?
            ));
            if let Some(systems) = value.get("active_systems").and_then(|v| v.as_array()) {
                for s in systems {
                    output.push_str(&format!(
                        "    - {} ({})\n",
                        s.get("system_id")?.as_str()?,
                        s.get("namespace")?.as_str()?
                    ));
                }
            }
            if let Some(latest) = value.get("latest_run").and_then(|v| v.as_str()) {
                output.push_str(&format!("  Latest Run: {}\n", latest));
            }
            output.push_str(&format!("  Runs: {}\n", value.get("run_count")?.as_u64()?));
            output.push_str(&format!(
                "  Change Sets: {}\n",
                value.get("change_set_count")?.as_u64()?
            ));
            output.push_str(&format!(
                "  Handoffs: {}\n",
                value.get("handoff_count")?.as_u64()?
            ));
            output.push_str(&format!(
                "  Failures: {}\n",
                value.get("failure_count")?.as_u64()?
            ));

            output.push_str("\nPaths:\n");
            let paths = value.get("paths")?;
            output.push_str(&format!("  Root: {}\n", value.get("root")?.as_str()?));
            output.push_str(&format!(
                "  Declarations: {}\n",
                paths.get("declarations_dir")?.as_str()?
            ));

            output.push_str("\nProvider Capabilities:\n");
            if let Some(providers) = value
                .get("provider_capabilities")
                .and_then(|v| v.as_array())
            {
                for p in providers {
                    let name = p.get("provider")?.as_str()?;
                    let status = p.get("status")?.as_str()?;
                    output.push_str(&format!("  - {}: {}\n", name, status));
                }
            }
        }
        "doctor" => {
            let ok = value.get("ok")?.as_bool()?;
            output.push_str(&format!(
                "Health Status: {}\n",
                if ok { "PASS" } else { "ISSUES FOUND" }
            ));
            output.push_str(&format!(
                "Canonical Objects: {}\n",
                value.get("canonical_object_count")?.as_u64()?
            ));
            output.push_str(&format!(
                "Indexed Objects: {}\n",
                value.get("indexed_object_count")?.as_u64()?
            ));

            if let Some(warnings) = value.get("warnings").and_then(|v| v.as_array()) {
                if !warnings.is_empty() {
                    output.push_str("\nWarnings:\n");
                    for w in warnings {
                        output.push_str(&format!("  - {}\n", w.as_str()?));
                    }
                }
            }
        }
        "review" => {
            output.push_str("Review Results:\n");
            output.push_str(&format!(
                "  Target Object: {}\n",
                value.get("target_object_id")?.as_str()?
            ));
            output.push_str(&format!("  Status: {}\n", value.get("status")?.as_str()?));
            output.push_str(&format!(
                "  Review Object: {}\n",
                value.get("review_object_id")?.as_str()?
            ));
        }
        "undo" => {
            output.push_str("Undo Results:\n");
            output.push_str(&format!(
                "  Undo Record: {}\n",
                value.get("undo_record_id")?.as_str()?
            ));
            let impact = value.get("impact")?;
            output.push_str(&format!(
                "  Objects Hidden: {}\n",
                impact.get("objects_hidden")?.as_u64()?
            ));
            output.push_str(&format!(
                "  Relations Hidden: {}\n",
                impact.get("relations_hidden")?.as_u64()?
            ));
        }
        "audit_failures" => {
            output.push_str("Failure Audit:\n");
            if let Some(failures) = value.get("failures").and_then(|v| v.as_array()) {
                for f in failures {
                    let fid = f.get("failure_id")?.as_str()?;
                    let etype = f.get("error_type")?.as_str()?;
                    let msg = f.get("message")?.as_str()?;
                    let tid = f.get("transition_id")?.as_str()?;
                    output.push_str(&format!(
                        "  - {} transition: {} error: {} \n    Message: {}\n",
                        fid, tid, etype, msg
                    ));
                }
            }
        }
        "run_list" => {
            output.push_str("Recent Runs:\n");
            if let Some(runs) = value.get("runs").and_then(|v| v.as_array()) {
                for r in runs {
                    let run_id = r.get("run_id")?.as_str()?;
                    let status = r.get("status")?.as_str()?;
                    let started = r.get("started_at")?.as_str()?;
                    output.push_str(&format!(
                        "  - {} [{}] (started {})\n",
                        run_id, status, started
                    ));
                }
            }
        }
        "run" => {
            let artifact = value.get("artifact")?;
            let related = value.get("related")?;
            output.push_str("Purpose: A run records the execution of a workflow system.\n");
            output.push_str(&format!("Status: {}\n", artifact.get("status")?.as_str()?));
            output.push_str(&format!(
                "Started At: {}\n",
                artifact.get("started_at")?.as_str()?
            ));
            if let Some(ended) = artifact.get("ended_at").and_then(|v| v.as_str()) {
                output.push_str(&format!("Ended At: {}\n", ended));
            }
            output.push_str("\nRelated Artifacts:\n");
            if let Some(assignments) = related.get("assignments").and_then(|v| v.as_array()) {
                output.push_str(&format!("  Assignments: {}\n", assignments.len()));
            }
            if let Some(change_sets) = related.get("change_sets").and_then(|v| v.as_array()) {
                output.push_str(&format!("  Change Sets: {}\n", change_sets.len()));
            }
            if let Some(handoffs) = related.get("handoffs").and_then(|v| v.as_array()) {
                output.push_str(&format!("  Handoffs: {}\n", handoffs.len()));
            }
            if let Some(failures) = related.get("failures").and_then(|v| v.as_array()) {
                output.push_str(&format!("  Failures: {}\n", failures.len()));
            }
        }
        "run_timeline" => {
            let timeline = value.get("timeline")?;
            output.push_str("Purpose: A run timeline shows the sequence of events and artifacts created during a run.\n");
            output.push_str(&format!("Status: {}\n", timeline.get("status")?.as_str()?));
            output.push_str(&format!(
                "Started At: {}\n",
                timeline.get("started_at")?.as_str()?
            ));
            if let Some(ended) = timeline.get("ended_at").and_then(|v| v.as_str()) {
                output.push_str(&format!("Ended At: {}\n", ended));
            }
            if let Some(events) = timeline.get("events").and_then(|v| v.as_array()) {
                output.push_str(&format!("\nEvents: {} events recorded\n", events.len()));
            }
            if let Some(assignments) = timeline.get("assignments").and_then(|v| v.as_array()) {
                output.push_str(&format!("Assignments: {}\n", assignments.len()));
            }
            if let Some(change_sets) = timeline.get("change_sets").and_then(|v| v.as_array()) {
                output.push_str(&format!("Change Sets: {}\n", change_sets.len()));
            }
            if let Some(handoffs) = timeline.get("handoffs").and_then(|v| v.as_array()) {
                output.push_str(&format!("Handoffs: {}\n", handoffs.len()));
            }
        }
        "run_artifacts" => {
            let artifacts = value.get("artifact")?;
            output.push_str(
                "Purpose: A run artifact inventory lists all durable records produced by a run.\n",
            );
            if let Some(v) = artifacts.get("assignments").and_then(|v| v.as_array()) {
                output.push_str(&format!(
                    "Assignments ({}): {}\n",
                    v.len(),
                    v.iter()
                        .map(|v| v.as_str().unwrap_or(""))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if let Some(v) = artifacts.get("change_sets").and_then(|v| v.as_array()) {
                output.push_str(&format!(
                    "Change Sets ({}): {}\n",
                    v.len(),
                    v.iter()
                        .map(|v| v.as_str().unwrap_or(""))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if let Some(v) = artifacts.get("handoffs").and_then(|v| v.as_array()) {
                output.push_str(&format!(
                    "Handoffs ({}): {}\n",
                    v.len(),
                    v.iter()
                        .map(|v| v.as_str().unwrap_or(""))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if let Some(v) = artifacts.get("failures").and_then(|v| v.as_array()) {
                output.push_str(&format!(
                    "Failures ({}): {}\n",
                    v.len(),
                    v.iter()
                        .map(|v| v.as_str().unwrap_or(""))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        "run_graph" => {
            let graph = value.get("graph")?;
            output.push_str(
                "Purpose: A relationship graph showing how artifacts in this run connect.\n\n",
            );
            output.push_str("Mermaid Diagram:\n");
            output.push_str("```mermaid\ngraph TD\n");
            if let Some(edges) = graph.get("edges").and_then(|v| v.as_array()) {
                for edge in edges {
                    let from = edge.get("from").and_then(|v| v.as_str()).unwrap_or("");
                    let to = edge.get("to").and_then(|v| v.as_str()).unwrap_or("");
                    let label = edge.get("label").and_then(|v| v.as_str()).unwrap_or("");
                    output.push_str(&format!("  {} -- \"{}\" --> {}\n", from, label, to));
                }
            }
            output.push_str("```\n");
        }
        "assignment" => {
            let artifact = value.get("artifact")?;
            let related = value.get("related")?;
            output.push_str("Purpose: An assignment represents a specific transition being executed by a runtime.\n");
            output.push_str(&format!(
                "Transition: {}\n",
                related.get("transition_id")?.as_str()?
            ));
            output.push_str(&format!("Status: {}\n", artifact.get("status")?.as_str()?));
            output.push_str(&format!("Run ID: {}\n", related.get("run_id")?.as_str()?));
            if let Some(cs) = related
                .get("completion_change_set_id")
                .and_then(|v| v.as_str())
            {
                output.push_str(&format!("Completed by Change Set: {}\n", cs));
            }
            if let Some(ho) = related.get("handoff_manifest_id").and_then(|v| v.as_str()) {
                output.push_str(&format!("Emitted Handoff: {}\n", ho));
            }
        }
        "change_set" => {
            let artifact = value.get("artifact")?;
            let related = value.get("related")?;
            output.push_str(
                "Purpose: A change set records the state changes produced by a transition.\n",
            );
            output.push_str(&format!(
                "Transition: {}\n",
                artifact.get("transition_id")?.as_str()?
            ));
            output.push_str(&format!("Run ID: {}\n", artifact.get("run_id")?.as_str()?));
            if let Some(aid) = related.get("assignment_id").and_then(|v| v.as_str()) {
                output.push_str(&format!("Produced for Assignment: {}\n", aid));
            }
            if let Some(ho) = related.get("handoff_manifest_id").and_then(|v| v.as_str()) {
                output.push_str(&format!("Linked to Handoff: {}\n", ho));
            }
        }
        "handoff" => {
            let artifact = value.get("artifact")?;
            let related = value.get("related")?;
            output.push_str("Purpose: A handoff defines the bounded surface for successor work.\n");
            output.push_str(&format!(
                "From Transition: {}\n",
                artifact.get("from_transition_id")?.as_str()?
            ));
            if let Some(to) = related.get("to_transition_id").and_then(|v| v.as_str()) {
                output.push_str(&format!("Intended Successor: {}\n", to));
            }
            output.push_str(&format!("Run ID: {}\n", related.get("run_id")?.as_str()?));
            output.push_str(&format!(
                "Source Change Set: {}\n",
                related.get("source_change_set_id")?.as_str()?
            ));
        }
        "failure" => {
            let artifact = value.get("artifact")?;
            let related = value.get("related")?;
            output.push_str("Purpose: A failure record persists a failed transition attempt for audit and repair.\n");
            output.push_str(&format!(
                "Transition: {}\n",
                artifact.get("transition_id")?.as_str()?
            ));
            output.push_str(&format!(
                "Error Type: {}\n",
                artifact.get("error_type")?.as_str()?
            ));
            output.push_str(&format!(
                "Message: {}\n",
                artifact.get("message")?.as_str()?
            ));
            output.push_str(&format!("Run ID: {}\n", artifact.get("run_id")?.as_str()?));
            output.push_str(&format!(
                "Assignment ID: {}\n",
                related.get("assignment_id")?.as_str()?
            ));
            if let Some(cs) = related.get("failed_change_set_id").and_then(|v| v.as_str()) {
                output.push_str(&format!("Failed Change Set: {}\n", cs));
            }
        }
        "report_generation" => {
            output.push_str(
                "Purpose: A command that generates a static HTML report for inspection.\n",
            );
            output.push_str(&format!(
                "Target Kind: {}\n",
                value.get("target_kind")?.as_str()?
            ));
            output.push_str(&format!(
                "Target ID: {}\n",
                value.get("target_id")?.as_str()?
            ));
            output.push_str(&format!(
                "Output Path: {}\n",
                value.get("output")?.as_str()?
            ));
        }
        "relation" => {
            let related = value.get("related")?;
            output.push_str("Purpose: A relation defines a directed link between two objects.\n");
            output.push_str(&format!(
                "Relation Type: {}\n",
                related.get("relation_type")?.as_str()?
            ));
            output.push_str(&format!(
                "Source: {}\n",
                related.get("source")?.get("id")?.as_str()?
            ));
            output.push_str(&format!(
                "Target: {}\n",
                related.get("target")?.get("id")?.as_str()?
            ));
            if let Some(mode) = related.get("creation_mode").and_then(|v| v.as_str()) {
                output.push_str(&format!("Creation Mode: {}\n", mode));
            }

            if let Some(auth) = related.get("authorization") {
                if !auth.is_null() {
                    output.push_str("\nAuthorization Trace:\n");
                    if let Some(endpoint) = auth.get("endpoint").and_then(|v| v.as_str()) {
                        output.push_str(&format!("  Authorizing Endpoint: {}\n", endpoint));
                    }
                    if let Some(class) = auth.get("class").and_then(|v| v.as_str()) {
                        output.push_str(&format!("  Authorizing Class: {}\n", class));
                    }
                    if let Some(authority) = auth.get("authority").and_then(|v| v.as_str()) {
                        output.push_str(&format!("  Configured Authority: {}\n", authority));
                    }
                    if let Some(direction) = auth.get("direction").and_then(|v| v.as_str()) {
                        output.push_str(&format!("  Rule Direction: {}\n", direction));
                    }
                }
            }
        }
        _ => return None,
    }

    if let Some(cmds) = next_commands {
        output.push_str("\nNext Inspection Steps:\n");
        for cmd in cmds {
            if let Some(c) = cmd.as_str() {
                output.push_str(&format!("  - {}\n", c));
            }
        }
    }

    Some(output)
}
