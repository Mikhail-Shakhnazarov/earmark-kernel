use crate::app::common::CliError;
use crate::app::graph::build_run_graph;
use crate::app::listing::{
    list_provider_records_by_run, load_run_record_by_id, run_related_artifacts,
};
use crate::app::loaders::load_handoff_by_id;
use crate::app::resolve::resolve_system_version_ref;
use earmark_index::DerivedIndex;
use earmark_store::CanonicalStore;

fn html_wrap(title: &str, content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Earmark Report: {title}</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&display=swap" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/mermaid/dist/mermaid.min.js"></script>
    <script>mermaid.initialize({{ startOnLoad: true, theme: 'dark' }});</script>
    <style>
        :root {{
            --bg-color: #0f172a;
            --card-bg: #1e293b;
            --text-color: #f1f5f9;
            --text-dim: #94a3b8;
            --primary: #38bdf8;
            --accent: #818cf8;
            --success: #10b981;
            --error: #ef4444;
            --warning: #f59e0b;
            --border: #334155;
        }}
        body {{
            font-family: 'Inter', sans-serif;
            background-color: var(--bg-color);
            color: var(--text-color);
            margin: 0;
            padding: 2rem;
            line-height: 1.5;
        }}
        .container {{
            max-width: 1000px;
            margin: 0 auto;
        }}
        header {{
            border-bottom: 1px solid var(--border);
            padding-bottom: 1.5rem;
            margin-bottom: 2rem;
        }}
        h1 {{
            margin: 0;
            font-size: 2rem;
            background: linear-gradient(to right, var(--primary), var(--accent));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        .summary-card {{
            background: var(--card-bg);
            border: 1px solid var(--border);
            border-radius: 0.75rem;
            padding: 1.5rem;
            margin-bottom: 2rem;
            box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
        }}
        .artifact-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
            gap: 1.5rem;
            margin-bottom: 2rem;
        }}
        .card {{
            background: var(--card-bg);
            border: 1px solid var(--border);
            border-radius: 0.75rem;
            padding: 1rem;
        }}
        .card h3 {{
            margin-top: 0;
            font-size: 1.1rem;
            color: var(--primary);
        }}
        .tag {{
            display: inline-block;
            padding: 0.25rem 0.5rem;
            border-radius: 0.375rem;
            font-size: 0.75rem;
            font-weight: 600;
            text-transform: uppercase;
        }}
        .tag-success {{ background: rgba(16, 185, 129, 0.2); color: var(--success); }}
        .tag-error {{ background: rgba(239, 68, 68, 0.2); color: var(--error); }}
        pre {{
            background: #000;
            padding: 1rem;
            border-radius: 0.5rem;
            overflow-x: auto;
            font-size: 0.875rem;
            border: 1px solid var(--border);
        }}
        .mermaid {{
            background: white;
            padding: 1rem;
            border-radius: 0.75rem;
            margin: 1.5rem 0;
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>Earmark Report: {title}</h1>
            <p style="color: var(--text-dim)">Generated at {now}</p>
        </header>
        {content}
    </div>
</body>
</html>"#,
        title = title,
        now = chrono::Utc::now().to_rfc3339(),
        content = content
    )
}

pub(crate) fn generate_run_report<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<String, CliError> {
    let ledger = load_run_record_by_id(store, run_id)?;
    let related = run_related_artifacts(store, run_id)?;
    let graph = build_run_graph(store, run_id)?;

    let mut content = String::new();
    if let Some(synthetic_change_sets) = related
        .get("synthetic_change_sets")
        .and_then(|v| v.as_array())
    {
        if !synthetic_change_sets.is_empty() {
            content.push_str(
                r#"<div class="summary-card" style="border-left: 4px solid var(--warning);">
            <h2>Synthetic Output Warning</h2>
            <p>This run includes change sets produced from synthetic mock provider output. Do not treat these artifacts as model-derived production evidence.</p>
        </div>"#,
            );
        }
    }
    content.push_str(&format!(
        r#"<div class="summary-card">
            <h2>Run Summary</h2>
            <p><strong>ID:</strong> {run_id}</p>
            <p><strong>Status:</strong> <span class="tag tag-{status_class}">{status}</span></p>
            <p><strong>Started:</strong> {started}</p>
            <p><strong>Ended:</strong> {ended}</p>
            <p><strong>Events:</strong> {events}</p>
        </div>"#,
        run_id = run_id,
        status = format!("{:?}", ledger.status).to_lowercase(),
        status_class = if matches!(ledger.status, earmark_core::RunStatus::Completed) {
            "success"
        } else {
            "error"
        },
        started = ledger.started_at,
        ended = ledger
            .ended_at
            .map(|d| d.to_rfc3339())
            .unwrap_or_else(|| "N/A".to_string()),
        events = ledger.events.len()
    ));

    let provider_records = list_provider_records_by_run(store, run_id)?;
    content.push_str(&format!(
        r#"<div class="summary-card">
            <h2>Why This Matters</h2>
            <p>This report is a durable audit artifact for run <code>{run_id}</code>. It captures execution status, lineage-relevant provider interactions, and failure outcomes so another engineer can review trust boundaries without re-running the workflow.</p>
            <p><strong>Provider Records:</strong> {provider_record_count}</p>
        </div>"#,
        run_id = run_id,
        provider_record_count = provider_records.len()
    ));

    content.push_str("<h2>Artifact Relationship Graph</h2>");
    content.push_str("<div class=\"mermaid\">\ngraph TD\n");
    if let Some(edges) = graph.get("edges").and_then(|v| v.as_array()) {
        for edge in edges {
            let from = edge.get("from").and_then(|v| v.as_str()).unwrap_or("");
            let to = edge.get("to").and_then(|v| v.as_str()).unwrap_or("");
            let label = edge.get("label").and_then(|v| v.as_str()).unwrap_or("");
            content.push_str(&format!("  {} -- \"{}\" --> {}\n", from, label, to));
        }
    }
    content.push_str("</div>");

    content.push_str("<h2>Timeline Events</h2>");
    content.push_str("<div class=\"summary-card\"><ul>");
    for event in &ledger.events {
        content.push_str(&format!(
            "<li><code>{ts}</code> - <strong>{kind}</strong>: {msg}</li>",
            ts = event.timestamp,
            kind = event.event_type,
            msg = event.message.as_deref().unwrap_or_default()
        ));
    }
    content.push_str("</ul></div>");

    content.push_str("<h2>Provider Records</h2>");
    content.push_str("<div class=\"artifact-grid\">");
    for record in &provider_records {
        let warning_count = record.advisory_warnings.len();
        let message = record.message.as_deref().unwrap_or("n/a");
        content.push_str(&format!(
            r#"<div class="card">
                <h3>{provider} / {model}</h3>
                <p><strong>Status:</strong> {status}</p>
                <p><strong>Record ID:</strong> <code>{record_id}</code></p>
                <p><strong>Warnings:</strong> {warning_count}</p>
                <p><strong>Message:</strong> {message}</p>
            </div>"#,
            provider = record.provider.as_str(),
            model = record.model.as_str(),
            status = format!("{:?}", record.status).to_lowercase(),
            record_id = record.record_id.as_str(),
            warning_count = warning_count,
            message = message
        ));
    }
    if provider_records.is_empty() {
        content.push_str(
            r#"<div class="card"><h3>No Provider Records</h3><p>This run did not emit provider record events.</p></div>"#,
        );
    }
    content.push_str("</div>");

    content.push_str("<h2>Failure Details</h2>");
    content.push_str("<div class=\"summary-card\"><ul>");
    if let Some(failures) = related.get("failures").and_then(|v| v.as_array()) {
        if failures.is_empty() {
            content.push_str("<li>No failures recorded.</li>");
        } else {
            for failure in failures {
                let transition_id = failure
                    .get("transition_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let error_type = failure
                    .get("error_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let message = failure
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("n/a");
                content.push_str(&format!(
                    "<li><strong>{}</strong> ({}) - {}</li>",
                    transition_id, error_type, message
                ));
            }
        }
    }
    content.push_str("</ul></div>");

    let visibility_warnings = ledger
        .events
        .iter()
        .filter_map(|event| event.message.as_deref())
        .filter(|message| {
            message.contains("visibility")
                || message.contains("work surface")
                || message.contains("compiled context")
                || message.contains("provider profile")
        })
        .collect::<Vec<_>>();
    content.push_str("<h2>Visibility Boundaries</h2>");
    content.push_str("<div class=\"summary-card\"><ul>");
    if visibility_warnings.is_empty() {
        content.push_str(
            "<li>No explicit visibility-boundary warnings were recorded for this run.</li>",
        );
    } else {
        for warning in visibility_warnings {
            content.push_str(&format!("<li>{}</li>", warning));
        }
    }
    content.push_str("</ul></div>");

    content.push_str("<h2>Artifact Inventory</h2>");
    content.push_str("<div class=\"artifact-grid\">");

    if let Some(assignments) = related.get("assignments").and_then(|v| v.as_array()) {
        for id in assignments {
            content.push_str(&format!(
                "<div class=\"card\"><h3>Assignment</h3><p><code>{}</code></p></div>",
                id.as_str().unwrap_or("")
            ));
        }
    }
    if let Some(change_sets) = related.get("change_sets").and_then(|v| v.as_array()) {
        for id in change_sets {
            content.push_str(&format!(
                "<div class=\"card\"><h3>Change Set</h3><p><code>{}</code></p></div>",
                id.as_str().unwrap_or("")
            ));
        }
    }
    if let Some(handoffs) = related.get("handoffs").and_then(|v| v.as_array()) {
        for id in handoffs {
            content.push_str(&format!(
                "<div class=\"card\"><h3>Handoff</h3><p><code>{}</code></p></div>",
                id.as_str().unwrap_or("")
            ));
        }
    }
    if let Some(failures) = related.get("failures").and_then(|v| v.as_array()) {
        for id in failures {
            content.push_str(&format!(
                "<div class=\"card\"><h3>Failure</h3><p><code>{}</code></p></div>",
                id.as_str().unwrap_or("")
            ));
        }
    }
    content.push_str("</div>");

    Ok(html_wrap(&format!("Run {}", run_id), &content))
}

pub(crate) fn generate_handoff_report<S: CanonicalStore>(
    store: &S,
    handoff_id: &str,
) -> Result<String, CliError> {
    let handoff = load_handoff_by_id(store, handoff_id)?;
    let mut content = String::new();
    content.push_str(&format!(
        r#"<div class="summary-card">
            <h2>Handoff Summary</h2>
            <p><strong>ID:</strong> {handoff_id}</p>
            <p><strong>From Transition:</strong> {from}</p>
            <p><strong>To Transition:</strong> {to}</p>
            <p><strong>Run ID:</strong> {run_id}</p>
        </div>"#,
        handoff_id = handoff_id,
        from = handoff.from_transition_id,
        to = handoff
            .to_transition_id
            .unwrap_or_else(|| "N/A".to_string()),
        run_id = handoff.run_id
    ));

    content.push_str("<h2>Continuation Constraints</h2>");
    content.push_str("<div class=\"summary-card\">");
    content.push_str("<p><strong>Allowed Input Classes:</strong> ");
    content.push_str(&handoff.allowed_input_classes.join(", "));
    content.push_str("</p>");
    content.push_str("<p><strong>Required Checks:</strong> ");
    content.push_str(
        &handoff
            .required_checks
            .iter()
            .map(|c| c.check_type.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    );
    content.push_str("</p>");
    content.push_str("</div>");

    content.push_str("<h2>Bounded Artifacts</h2>");
    content.push_str("<div class=\"artifact-grid\">");
    for oid in &handoff.newly_created_object_ids {
        content.push_str(&format!(
            "<div class=\"card\"><h3>Created Object</h3><p><code>{}</code></p></div>",
            oid.as_str()
        ));
    }
    for oid in &handoff.root_object_ids {
        content.push_str(&format!(
            "<div class=\"card\"><h3>Root Object</h3><p><code>{}</code></p></div>",
            oid.as_str()
        ));
    }
    content.push_str("</div>");

    Ok(html_wrap(&format!("Handoff {}", handoff_id), &content))
}

pub(crate) fn generate_system_report<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    system_id: &str,
) -> Result<String, CliError> {
    let system_ref = resolve_system_version_ref(index, system_id)?;
    let system_obj = store.read_version(&system_ref)?;
    let system: earmark_core::SystemDefinition = serde_json::from_slice(&system_obj.payload.bytes)?;

    let mut content = String::new();
    content.push_str(&format!(
        r#"<div class="summary-card">
            <h2>System Summary</h2>
            <p><strong>ID:</strong> {system_id}</p>
            <p><strong>Title:</strong> {title}</p>
            <p><strong>Namespace:</strong> {namespace}</p>
            <p><strong>Description:</strong> {description}</p>
        </div>"#,
        system_id = system.system_id,
        title = system.title,
        namespace = system.namespace,
        description = system.description.unwrap_or_else(|| "N/A".to_string())
    ));

    content.push_str("<h2>Declaration Inventory</h2>");
    content.push_str("<div class=\"artifact-grid\">");
    content.push_str(&format!(
        "<div class=\"card\"><h3>Classes</h3><p>{}</p></div>",
        system.classes.len()
    ));
    content.push_str(&format!(
        "<div class=\"card\"><h3>Instructions</h3><p>{}</p></div>",
        system.instructions.len()
    ));
    content.push_str(&format!(
        "<div class=\"card\"><h3>Workflows</h3><p>{}</p></div>",
        system.workflows.len()
    ));
    content.push_str(&format!(
        "<div class=\"card\"><h3>Provider Profiles</h3><p>{}</p></div>",
        system.provider_profiles.len()
    ));
    content.push_str("</div>");

    Ok(html_wrap(&format!("System {}", system_id), &content))
}
