use std::{fs, path::PathBuf};

pub(crate) mod common;
pub(crate) use common::CliError;
mod bootstrap;
mod commands;
pub(crate) use commands::declarations::{
    explain_declaration_file, register_declaration_file, resolve_version_ref,
    resolve_workflow_version_ref, validate_declaration_file,
};
mod dispatch;

use crate::cli::*;
use crate::output;
use clap_complete::{generate, shells};
use earmark_core::{
    HeaderValue, Kind, ObjectRef, VersionRef,
};
use earmark_index::DerivedIndex;
use earmark_store::{
    CanonicalStore, GitCanonicalStore, PayloadEncoding, StoredObject, WorkspaceLayout,
};
use serde_json::json;

pub fn run(cli: Cli) -> Result<(), common::CliError> {
    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = command_for_completions();
        match shell {
            CompletionShell::Bash => generate(shells::Bash, &mut cmd, "em", &mut std::io::stdout()),
            CompletionShell::Zsh => generate(shells::Zsh, &mut cmd, "em", &mut std::io::stdout()),
            CompletionShell::Fish => generate(shells::Fish, &mut cmd, "em", &mut std::io::stdout()),
        }
        return Ok(());
    }

    let bootstrapped = bootstrap::bootstrap(&cli)?;
    let ctx = common::CommandContext {
        store: &bootstrapped.store,
        index: &bootstrapped.index,
        config: &bootstrapped.config,
        as_json: bootstrapped.as_json,
        provider_registry: &bootstrapped.provider_registry,
    };

    let command_name = common::command_family_name(&cli.command);
    let started = std::time::Instant::now();

    tracing::debug!(root = %bootstrapped.root.display(), command = %command_name, "starting command");

    let result = dispatch::dispatch(&ctx, cli);
    crate::metrics::record_command_result(command_name, result.is_ok(), started.elapsed());
    result
}

fn template_contents_for_kind(kind: DeclarationKind) -> &'static str {
    match kind {
        DeclarationKind::Class => include_str!("../../../templates/classes/class.yaml"),
        DeclarationKind::Instruction => {
            include_str!("../../../templates/instructions/instruction.md")
        }
        DeclarationKind::StandingPolicy => {
            include_str!("../../../templates/standing_policies/standing_policy.yaml")
        }
        DeclarationKind::CompiledContext => {
            include_str!("../../../templates/compiled_contexts/compiled_context.yaml")
        }
        DeclarationKind::ProviderProfile => {
            include_str!("../../../templates/provider_profiles/provider_profile.yaml")
        }
        DeclarationKind::Workflow => include_str!("../../../templates/workflows/workflow.yaml"),
        DeclarationKind::System => {
            include_str!("../../../templates/systems/system_path_manifest.yaml")
        }
    }
}

fn default_output_path(root: &std::path::Path, kind: DeclarationKind, name: &str) -> PathBuf {
    let (dir, ext) = match kind {
        DeclarationKind::Class => ("declarations/classes", "yaml"),
        DeclarationKind::Instruction => ("declarations/instructions", "md"),
        DeclarationKind::StandingPolicy => ("declarations/standing_policies", "yaml"),
        DeclarationKind::CompiledContext => ("declarations/compiled_contexts", "yaml"),
        DeclarationKind::ProviderProfile => ("declarations/provider_profiles", "yaml"),
        DeclarationKind::Workflow => ("declarations/workflows", "yaml"),
        DeclarationKind::System => ("declarations/systems", "yaml"),
    };
    root.join(dir).join(format!("{name}.{ext}"))
}

fn scaffold_declaration(
    root: &std::path::Path,
    kind: DeclarationKind,
    name: &str,
    explicit_path: Option<&PathBuf>,
    force: bool,
) -> Result<PathBuf, CliError> {
    let mut body = template_contents_for_kind(kind).to_string();
    body = body
        .replace("your_class_name", name)
        .replace("your_instruction_name", name)
        .replace("your_standing_policy", name)
        .replace("your_compiled_context", name)
        .replace("your_provider_profile", name)
        .replace("your_workflow", name)
        .replace("your_system", name);

    let out_path = explicit_path
        .cloned()
        .unwrap_or_else(|| default_output_path(root, kind, name));
    if out_path.exists() && !force {
        return Err(CliError::argument(format!(
            "target already exists: {} (pass --force to overwrite)",
            out_path.display()
        )));
    }
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out_path, body)?;
    Ok(out_path)
}

fn collect_paths_with_extensions(
    root: &std::path::Path,
    extensions: &[&str],
    out: &mut Vec<String>,
) -> Result<(), common::CliError> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_paths_with_extensions(&path, extensions, out)?;
            continue;
        }
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if !extensions
            .iter()
            .any(|candidate| ext.eq_ignore_ascii_case(candidate))
        {
            continue;
        }
        out.push(path.display().to_string());
    }
    Ok(())
}

fn resolve_object_ref<S: CanonicalStore>(
    store: &S,
    object_id: &str,
) -> Result<ObjectRef, CliError> {
    let head = store
        .read_head(&earmark_core::ObjectId::parse(object_id.to_string())?)?
        .ok_or_else(|| CliError::not_found(format!("object not found: {}", object_id)))?;
    Ok(head.object_ref())
}

fn resolve_run_id<S: CanonicalStore>(store: &S, run_id: &str) -> Result<String, CliError> {
    if run_id == "latest" {
        let ledgers = list_run_records(store)?;
        return ledgers
            .last()
            .map(|l| l.run_id.clone())
            .ok_or_else(|| CliError::not_found("no runs found".to_string()));
    }
    Ok(run_id.to_string())
}

fn resolve_optional_run_id<S: CanonicalStore>(
    store: &S,
    run_id: Option<String>,
) -> Result<Option<String>, CliError> {
    match run_id {
        Some(id) => Ok(Some(resolve_run_id(store, &id)?)),
        None => Ok(None),
    }
}

fn list_run_records<S: CanonicalStore>(
    store: &S,
) -> Result<Vec<earmark_core::RunRecord>, CliError> {
    let mut ledgers = Vec::new();
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::RunRecord {
            continue;
        }
        let ledger: earmark_core::RunRecord = serde_json::from_slice(&object.payload.bytes)?;
        ledgers.push(ledger);
    }
    ledgers.sort_by(|a, b| {
        a.started_at
            .cmp(&b.started_at)
            .then_with(|| a.run_id.cmp(&b.run_id))
    });
    Ok(ledgers)
}

fn load_run_record_by_id<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<earmark_core::RunRecord, CliError> {
    let ledgers = list_run_records(store)?;
    if run_id == "latest" {
        return ledgers
            .last()
            .cloned()
            .ok_or_else(|| CliError::not_found("no runs found".to_string()));
    }
    for ledger in ledgers {
        if ledger.run_id == run_id {
            return Ok(ledger);
        }
    }
    Err(CliError::not_found(format!("run not found: {}", run_id)))
}

fn list_assignments<S: CanonicalStore>(
    store: &S,
) -> Result<Vec<earmark_core::TransitionAssignment>, CliError> {
    let mut assignments = Vec::new();
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::TransitionAssignment {
            continue;
        }
        if let Some(head_ref) = store.read_head_ref(&object.envelope.id)? {
            if head_ref.version_id != object.envelope.version_id {
                continue;
            }
        }
        let assignment: earmark_core::TransitionAssignment =
            serde_json::from_slice(&object.payload.bytes)?;
        assignments.push(assignment);
    }
    Ok(assignments)
}

fn list_assignments_by_run<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<Vec<earmark_core::TransitionAssignment>, CliError> {
    Ok(list_assignments(store)?
        .into_iter()
        .filter(|assignment| assignment.run_id == run_id)
        .collect())
}

fn list_change_sets<S: CanonicalStore>(
    store: &S,
) -> Result<Vec<earmark_core::ChangeSet>, CliError> {
    let mut change_sets = Vec::new();
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::ChangeSet {
            continue;
        }
        let change_set: earmark_core::ChangeSet = serde_json::from_slice(&object.payload.bytes)?;
        change_sets.push(change_set);
    }
    Ok(change_sets)
}

fn list_change_sets_by_run<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<Vec<earmark_core::ChangeSet>, CliError> {
    Ok(list_change_sets(store)?
        .into_iter()
        .filter(|change_set| change_set.run_id == run_id)
        .collect())
}

fn list_handoffs<S: CanonicalStore>(
    store: &S,
) -> Result<Vec<earmark_core::HandoffManifest>, CliError> {
    let mut handoffs = Vec::new();
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::HandoffManifest {
            continue;
        }
        let handoff: earmark_core::HandoffManifest = serde_json::from_slice(&object.payload.bytes)?;
        handoffs.push(handoff);
    }
    Ok(handoffs)
}

fn list_handoffs_by_run<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<Vec<earmark_core::HandoffManifest>, CliError> {
    Ok(list_handoffs(store)?
        .into_iter()
        .filter(|handoff| handoff.run_id == run_id)
        .collect())
}

fn list_failure_objects<S: CanonicalStore>(
    store: &S,
) -> Result<Vec<(String, earmark_core::TransformationFailure)>, CliError> {
    let mut failures = Vec::new();
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::TransformationFailure {
            continue;
        }
        let failure: earmark_core::TransformationFailure =
            serde_json::from_slice(&object.payload.bytes)?;
        failures.push((object.envelope.id.as_str().to_string(), failure));
    }
    Ok(failures)
}

fn list_failures_by_run<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<Vec<String>, CliError> {
    Ok(list_failure_objects(store)?
        .into_iter()
        .filter(|(_, failure)| failure.run_id == run_id)
        .map(|(id, _)| id)
        .collect())
}

fn list_failures<S: CanonicalStore>(
    store: &S,
    run_id: Option<&str>,
    transition_id: Option<&str>,
) -> Result<Vec<serde_json::Value>, CliError> {
    let mut failures = Vec::new();
    for (failure_id, failure) in list_failure_objects(store)? {
        if let Some(run_id) = run_id {
            if failure.run_id != run_id {
                continue;
            }
        }
        if let Some(transition_id) = transition_id {
            if failure.transition_id != transition_id {
                continue;
            }
        }
        failures.push(json!({
            "failure_id": failure_id,
            "run_id": failure.run_id,
            "transition_id": failure.transition_id,
            "assignment_id": failure.assignment_id.as_str(),
            "error_type": failure.error_type,
            "message": failure.message,
            "created_at": failure.created_at,
        }));
    }
    Ok(failures)
}

fn run_related_artifacts<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<serde_json::Value, CliError> {
    let assignments = list_assignments_by_run(store, run_id)?
        .into_iter()
        .map(|assignment| assignment.id.as_str().to_string())
        .collect::<Vec<_>>();
    let change_sets_full = list_change_sets_by_run(store, run_id)?;
    let change_sets = change_sets_full
        .iter()
        .map(|change_set| change_set.id.as_str().to_string())
        .collect::<Vec<_>>();
    let mut synthetic_change_sets = Vec::new();
    for change_set in &change_sets_full {
        let (synthetic, synthetic_source) = change_set_synthetic_marker(store, change_set)?;
        if synthetic {
            synthetic_change_sets.push(json!({
                "change_set_id": change_set.id.as_str(),
                "synthetic_source": synthetic_source,
            }));
        }
    }
    let handoffs = list_handoffs_by_run(store, run_id)?
        .into_iter()
        .map(|handoff| handoff.id.as_str().to_string())
        .collect::<Vec<_>>();
    let failures = list_failures_by_run(store, run_id)?
        .into_iter()
        .collect::<Vec<_>>();
    Ok(json!({
        "assignments": assignments,
        "change_sets": change_sets,
        "synthetic_change_sets": synthetic_change_sets,
        "handoffs": handoffs,
        "failures": failures,
    }))
}

pub(crate) fn load_current_assignment_by_id<S: CanonicalStore>(
    store: &S,
    assignment_id: &str,
) -> Result<earmark_core::TransitionAssignment, CliError> {
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::TransitionAssignment {
            continue;
        }
        let assignment: earmark_core::TransitionAssignment =
            serde_json::from_slice(&object.payload.bytes)?;
        if assignment.id.as_str() != assignment_id {
            continue;
        }
        if let Some(head_ref) = store.read_head_ref(&object.envelope.id)? {
            if head_ref.version_id == object.envelope.version_id {
                return Ok(assignment);
            }
        }
    }
    Err(CliError::not_found(format!(
        "assignment not found: {}",
        assignment_id
    )))
}

pub(crate) fn load_change_set_by_id<S: CanonicalStore>(
    store: &S,
    change_set_id: &str,
) -> Result<earmark_core::ChangeSet, CliError> {
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::ChangeSet {
            continue;
        }
        let change_set: earmark_core::ChangeSet = serde_json::from_slice(&object.payload.bytes)?;
        if change_set.id.as_str() == change_set_id {
            return Ok(change_set);
        }
    }
    Err(CliError::not_found(format!(
        "change set not found: {}",
        change_set_id
    )))
}

pub(crate) fn change_set_synthetic_marker<S: CanonicalStore>(
    store: &S,
    change_set: &earmark_core::ChangeSet,
) -> Result<(bool, Option<String>), CliError> {
    for object_id in &change_set.created_object_ids {
        let Some(stored) = store.read_head(object_id)? else {
            continue;
        };
        let synthetic = matches!(
            stored.envelope.headers.get("synthetic"),
            Some(HeaderValue::Bool(true))
        );
        if !synthetic {
            continue;
        }
        let source = match stored.envelope.headers.get("synthetic_source") {
            Some(HeaderValue::String(value)) => Some(value.clone()),
            _ => None,
        };
        return Ok((true, source));
    }
    Ok((false, None))
}

pub(crate) fn load_handoff_by_id<S: CanonicalStore>(
    store: &S,
    handoff_id: &str,
) -> Result<earmark_core::HandoffManifest, CliError> {
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::HandoffManifest {
            continue;
        }
        let handoff: earmark_core::HandoffManifest = serde_json::from_slice(&object.payload.bytes)?;
        if handoff.id.as_str() == handoff_id {
            return Ok(handoff);
        }
    }
    Err(CliError::not_found(format!(
        "handoff not found: {}",
        handoff_id
    )))
}

pub(crate) fn load_failure_by_id<S: CanonicalStore>(
    store: &S,
    failure_id: &str,
) -> Result<earmark_core::TransformationFailure, CliError> {
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::TransformationFailure {
            continue;
        }
        if object.envelope.id.as_str() == failure_id {
            let failure: earmark_core::TransformationFailure =
                serde_json::from_slice(&object.payload.bytes)?;
            return Ok(failure);
        }
    }
    Err(CliError::not_found(format!(
        "failure not found: {}",
        failure_id
    )))
}

fn load_relation_object_by_id<S: CanonicalStore>(
    store: &S,
    relation_id: &str,
) -> Result<StoredObject, CliError> {
    let id = earmark_core::ObjectId::parse(relation_id)
        .map_err(|_| CliError::argument(format!("invalid relation ID: {}", relation_id)))?;
    let found = store
        .read_head(&id)?
        .ok_or_else(|| CliError::not_found(format!("relation not found: {}", relation_id)))?;
    if found.envelope.kind != Kind::Relation {
        return Err(CliError::argument(format!(
            "object {} is not a relation",
            relation_id
        )));
    }
    Ok(found)
}

pub(crate) fn resolve_system_version_ref(
    index: &DerivedIndex,
    system_id: &str,
) -> Result<VersionRef, CliError> {
    let found = index.find_system_definition(system_id)?.ok_or_else(|| {
        CliError::not_found(format!("system definition not found: {}", system_id))
    })?;
    Ok(VersionRef::new(
        earmark_core::ObjectId::parse(found.0)?,
        earmark_core::VersionId::parse(found.1)?,
    ))
}

pub(crate) fn mirror_surface(
    store: &GitCanonicalStore,
    object: &StoredObject,
) -> Result<(), common::CliError> {
    let (dir, ext) = match &object.envelope.kind {
        Kind::Instruction => (
            store.declarations_dir().join("instructions"),
            object.payload.format.extension(),
        ),
        Kind::Workflow => (
            store.declarations_dir().join("workflows"),
            object.payload.format.extension(),
        ),
        Kind::Policy => (
            store.declarations_dir().join("standing_policies"),
            object.payload.format.extension(),
        ),
        Kind::CompiledContextTemplate => (
            store.declarations_dir().join("compiled_contexts"),
            object.payload.format.extension(),
        ),
        Kind::ProviderProfile => (
            store.declarations_dir().join("provider_profiles"),
            object.payload.format.extension(),
        ),
        Kind::SystemDefinition => (
            store.declarations_dir().join("systems"),
            object.payload.format.extension(),
        ),
        Kind::Object | Kind::Review
            if matches!(object.payload.format, PayloadEncoding::Markdown) =>
        {
            (store.corpus_dir(), object.payload.format.extension())
        }
        _ => (
            store
                .root()
                .join(".earmark")
                .join("canonical")
                .join("mirrors"),
            object.payload.format.extension(),
        ),
    };

    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.{}", object.envelope.id.as_str(), ext));
    fs::write(path, &object.payload.bytes)?;
    Ok(())
}

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

pub(crate) fn next_commands_for_run(run_id: &str) -> Vec<String> {
    vec![
        format!("em run explain {}", run_id),
        format!("em run timeline {}", run_id),
        format!("em run artifacts {}", run_id),
        format!("em run graph {}", run_id),
        format!("em failure list --run-id {}", run_id),
    ]
}

pub(crate) fn next_commands_for_assignment(
    assignment: &earmark_core::TransitionAssignment,
) -> Vec<String> {
    let mut commands = vec![format!("em run explain {}", assignment.run_id)];
    if let Some(hid) = &assignment.handoff_manifest_id {
        commands.push(format!("em handoff explain {}", hid.as_str()));
    }
    if let Some(did) = &assignment.completion_change_set_id {
        commands.push(format!("em change-set explain {}", did.as_str()));
    }
    commands.push(format!("em run timeline {}", assignment.run_id));
    commands
}

pub(crate) fn next_commands_for_change_set(change_set: &earmark_core::ChangeSet) -> Vec<String> {
    let mut commands = vec![format!("em run explain {}", change_set.run_id)];
    if let Some(hid) = &change_set.handoff_manifest_id {
        commands.push(format!("em handoff explain {}", hid.as_str()));
    }
    if let Some(aid) = &change_set.assignment_id {
        commands.push(format!("em assignment explain {}", aid.as_str()));
    }
    commands.push(format!("em run timeline {}", change_set.run_id));
    commands
}

pub(crate) fn next_commands_for_handoff(handoff: &earmark_core::HandoffManifest) -> Vec<String> {
    let mut commands = vec![format!("em run explain {}", handoff.run_id)];
    if let Some(transition_id) = &handoff.to_transition_id {
        commands.push(format!(
            "em workflow run <workflow_id> --system-id <system_id> --handoff {} # successor {}",
            handoff.id.as_str(),
            transition_id
        ));
    } else {
        commands.push(format!(
            "em workflow run <workflow_id> --system-id <system_id> --handoff {}",
            handoff.id.as_str()
        ));
    }
    commands.push(format!("em run timeline {}", handoff.run_id));
    commands
}

pub(crate) fn next_commands_for_failure(
    failure_id: &str,
    failure: &earmark_core::TransformationFailure,
) -> Vec<String> {
    let mut commands = vec![
        format!("em failure show {}", failure_id),
        format!("em run explain {}", failure.run_id),
    ];
    if let Some(delta_id) = &failure.failed_change_set_id {
        commands.push(format!("em change-set explain {}", delta_id.as_str()));
    }
    commands.push(format!(
        "em assignment explain {}",
        failure.assignment_id.as_str()
    ));
    commands.push(format!("em run timeline {}", failure.run_id));
    commands
}

pub(crate) fn emit(as_json: bool, value: serde_json::Value) {
    if as_json {
        output::emit_json_envelope(value);
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
                if ok { "✅ PASS" } else { "❌ ISSUES FOUND" }
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
                        output.push_str(&format!("  ⚠️ {}\n", w.as_str()?));
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

fn generate_run_report<S: CanonicalStore>(store: &S, run_id: &str) -> Result<String, CliError> {
    // Note: run_id must be already resolved here (not "latest")
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

fn generate_handoff_report<S: CanonicalStore>(
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

fn generate_system_report<S: CanonicalStore>(
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
