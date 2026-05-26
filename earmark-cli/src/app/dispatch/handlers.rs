use crate::app::commands::declarations::{
    explain_declaration_file, register_declaration_file, validate_declaration_file,
};
use crate::app::common::{CliError, CommandContext};
use crate::app::emitter::emit;
use crate::app::graph::build_run_graph;
use crate::app::listing::{
    list_assignments, list_change_sets, list_failure_objects, list_failures, list_handoffs,
    list_run_records, load_run_record_by_id, run_related_artifacts,
};
use crate::app::loaders::{
    change_set_synthetic_marker, load_change_set_by_id, load_current_assignment_by_id,
    load_failure_by_id, load_handoff_by_id, load_relation_object_by_id,
};
use crate::app::reports::{generate_handoff_report, generate_run_report, generate_system_report};
use crate::app::resolve::{resolve_optional_run_id, resolve_run_id};
use crate::app::scaffold::{collect_paths_with_extensions, scaffold_declaration};
use crate::app::suggestions::{
    next_commands_for_assignment, next_commands_for_change_set, next_commands_for_failure,
    next_commands_for_handoff, next_commands_for_run,
};
use crate::cli::*;
use earmark_core::Kind;
use earmark_index::DerivedIndex;
use earmark_store::{ObjectStore, StoreScanner, WorkspaceLayout, WorkspaceLayoutStatus};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;

pub(crate) fn handle_run(ctx: &mut CommandContext, command: RunCommand) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    match command.action {
        RunAction::List => {
            let runs = list_run_records(store)?;
            let summaries = runs
                .into_iter()
                .map(|run| {
                    json!({
                        "run_id": run.run_id,
                        "status": format!("{:?}", run.status).to_lowercase(),
                        "event_count": run.events.len(),
                        "started_at": run.started_at,
                        "ended_at": run.ended_at,
                    })
                })
                .collect::<Vec<_>>();
            emit(
                as_json,
                json!({
                    "kind": "run_list",
                    "summary": format!("{} runs found", summaries.len()),
                    "runs": summaries,
                    "next_commands": ["em run show <run_id>", "em run timeline <run_id>"],
                }),
            );
        }
        RunAction::Show { run_id } => {
            let rid = resolve_run_id(store, &run_id)?;
            let ledger = load_run_record_by_id(store, &rid)?;
            emit(as_json, serde_json::to_value(ledger)?);
        }
        RunAction::Timeline { run_id } => {
            let rid = resolve_run_id(store, &run_id)?;
            let mut ledger = load_run_record_by_id(store, &rid)?;
            let resolved_id = ledger.run_id.clone();
            ledger.events.sort_by_key(|event| event.timestamp);
            emit(
                as_json,
                json!({
                    "kind": "run_timeline",
                    "id": resolved_id,
                    "summary": format!("{} events across run {}", ledger.events.len(), resolved_id),
                    "timeline": {
                        "status": format!("{:?}", ledger.status).to_lowercase(),
                        "started_at": ledger.started_at,
                        "ended_at": ledger.ended_at,
                        "events": ledger.events,
                        "assignments": ledger.assignments,
                        "change_sets": ledger.change_sets,
                        "handoffs": ledger.manifests,
                    },
                    "related": run_related_artifacts(store, &resolved_id)?,
                    "next_commands": next_commands_for_run(resolved_id.as_str()),
                }),
            );
        }
        RunAction::Artifacts { run_id } => {
            let rid = resolve_run_id(store, &run_id)?;
            let ledger = load_run_record_by_id(store, &rid)?;
            let resolved_id = ledger.run_id.clone();
            emit(
                as_json,
                json!({
                    "kind": "run_artifacts",
                    "id": resolved_id,
                    "summary": format!("artifacts for run {}", resolved_id),
                    "artifact": run_related_artifacts(store, &resolved_id)?,
                    "next_commands": next_commands_for_run(resolved_id.as_str()),
                }),
            );
        }
        RunAction::Explain { run_id } => {
            let rid = resolve_run_id(store, &run_id)?;
            let ledger = load_run_record_by_id(store, &rid)?;
            let resolved_id = ledger.run_id.clone();
            emit(
                as_json,
                json!({
                    "kind": "run",
                    "id": resolved_id,
                    "summary": format!("run {} is {}", resolved_id, format!("{:?}", ledger.status).to_lowercase()),
                    "artifact": ledger,
                    "related": run_related_artifacts(store, &resolved_id)?,
                    "next_commands": next_commands_for_run(resolved_id.as_str()),
                }),
            );
        }
        RunAction::Graph { run_id } => {
            let rid = resolve_run_id(store, &run_id)?;
            let ledger = load_run_record_by_id(store, &rid)?;
            let resolved_id = ledger.run_id.clone();
            emit(
                as_json,
                json!({
                    "kind": "run_graph",
                    "id": resolved_id,
                    "summary": format!("relationship graph for run {}", resolved_id),
                    "graph": build_run_graph(store, &resolved_id)?,
                    "next_commands": next_commands_for_run(resolved_id.as_str()),
                }),
            );
        }
    }
    Ok(())
}

pub(crate) fn handle_declare(
    ctx: &mut CommandContext,
    command: DeclareCommand,
) -> Result<(), CliError> {
    let store = ctx.store;
    let index = &mut ctx.index;
    let as_json = ctx.as_json;
    let actor = ctx.actor;

    match command.action {
        DeclareAction::Validate(args) => {
            let summary = validate_declaration_file(store, args.kind, &args.path)?;
            emit(
                as_json,
                json!({
                    "action": "validate",
                    "kind": args.kind.as_str(),
                    "path": args.path.display().to_string(),
                    "summary": summary,
                }),
            );
        }
        DeclareAction::Explain(args) => {
            let explanation = explain_declaration_file(store, args.kind, &args.path)?;
            emit(
                as_json,
                json!({
                    "action": "explain",
                    "kind": args.kind.as_str(),
                    "path": args.path.display().to_string(),
                    "explanation": explanation,
                }),
            );
        }
        DeclareAction::New(args) => {
            let output_path = scaffold_declaration(
                store.root(),
                args.kind,
                &args.name,
                args.path.as_ref(),
                args.force,
            )?;
            emit(
                as_json,
                json!({
                    "action": "new",
                    "kind": args.kind.as_str(),
                    "name": args.name,
                    "path": output_path.display().to_string(),
                    "next_commands": [
                        format!("em declare validate --kind {} {}", args.kind.as_str(), output_path.display()),
                        format!("em declare explain --kind {} {}", args.kind.as_str(), output_path.display()),
                    ],
                }),
            );
        }
        DeclareAction::Register(args) => {
            tracing::info!(kind = %args.kind.as_str(), path = %args.path.display(), "registering declaration");
            let version_ref = register_declaration_file(
                store,
                index.as_mut(),
                args.kind,
                &args.path,
                None,
                actor,
            )?;
            if matches!(args.kind, DeclarationKind::System) {
                let idx = index
                    .as_mut()
                    .ok_or_else(|| CliError::WorkspaceNotInitialized {
                        status: WorkspaceLayoutStatus {
                            root_exists: false,
                            git_exists: false,
                            manifest_exists: false,
                            canonical_dir_exists: false,
                            objects_dir_exists: false,
                            payloads_dir_exists: false,
                            heads_dir_exists: false,
                            derived_dir_exists: false,
                            work_surfaces_dir_exists: false,
                            declarations_dir_exists: false,
                            corpus_dir_exists: false,
                        },
                    })?;
                idx.rebuild_from_store(store)?;
            }
            emit(
                as_json,
                json!({
                    "action": "register",
                    "kind": args.kind.as_str(),
                    "path": args.path.display().to_string(),
                    "object_id": version_ref.id.as_str(),
                    "version_id": version_ref.version_id.as_str(),
                }),
            );
        }
        DeclareAction::ListExamples => {
            let examples_dir = store
                .root()
                .join("docs")
                .join("declarations")
                .join("examples");
            let mut examples = Vec::new();
            if examples_dir.exists() {
                collect_paths_with_extensions(
                    &examples_dir,
                    &["yaml", "yml", "md"],
                    &mut examples,
                )?;
            }
            examples.sort();

            let summary = if examples.is_empty() {
                "No workspace-local declaration examples found under docs/declarations/examples"
                    .to_string()
            } else {
                format!("{} declaration examples found", examples.len())
            };

            let next_commands = if examples.is_empty() {
                vec![]
            } else {
                vec![
                    "em declare validate --kind class docs/declarations/examples/classes/finding.yaml".to_string(),
                    "em declare explain --kind workflow docs/declarations/examples/workflows/source_to_finding.yaml".to_string(),
                ]
            };

            emit(
                as_json,
                json!({
                    "summary": summary,
                    "examples_root": examples_dir.display().to_string(),
                    "examples": examples,
                    "next_commands": next_commands,
                }),
            );
        }
    }
    Ok(())
}

pub(crate) fn handle_assignment(
    ctx: &mut CommandContext,
    command: AssignmentCommand,
) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    match command.action {
        AssignmentAction::Show { assignment_id } => {
            let assignment = load_current_assignment_by_id(store, &assignment_id)?;
            emit(as_json, serde_json::to_value(assignment)?);
        }
        AssignmentAction::Explain { assignment_id } => {
            let assignment = load_current_assignment_by_id(store, &assignment_id)?;
            emit(
                as_json,
                json!({
                    "kind": "assignment",
                    "id": assignment_id,
                    "summary": format!("assignment {} in status {}", assignment.id.as_str(), format!("{:?}", assignment.status).to_lowercase()),
                    "artifact": assignment.clone(),
                    "related": {
                        "run_id": assignment.run_id.clone(),
                        "transition_id": assignment.transition_id.clone(),
                        "completion_change_set_id": assignment.completion_change_set_id.as_ref().map(|id| id.as_str().to_string()),
                        "handoff_manifest_id": assignment.handoff_manifest_id.as_ref().map(|id| id.as_str().to_string()),
                    },
                    "next_commands": next_commands_for_assignment(&assignment),
                }),
            );
        }
        AssignmentAction::List { run_id, status } => {
            let run_id = resolve_optional_run_id(store, run_id)?;
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
                if let Some(rid) = &run_id {
                    if assignment.run_id != *rid {
                        continue;
                    }
                }
                if let Some(status_str) = &status {
                    if format!("{:?}", assignment.status).to_lowercase()
                        != status_str.to_lowercase()
                    {
                        continue;
                    }
                }
                assignments.push(assignment);
            }
            emit(as_json, serde_json::to_value(assignments)?);
        }
    }
    Ok(())
}

pub(crate) fn handle_change_set(
    ctx: &mut CommandContext,
    command: ChangeSetCommand,
) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    match command.action {
        ChangeSetAction::Show { change_set_id } => {
            let change_set = load_change_set_by_id(store, &change_set_id)?;
            emit(as_json, serde_json::to_value(change_set)?);
        }
        ChangeSetAction::Explain { change_set_id } => {
            let change_set = load_change_set_by_id(store, &change_set_id)?;
            let (synthetic, synthetic_source) = change_set_synthetic_marker(store, &change_set)?;
            emit(
                as_json,
                json!({
                    "kind": "change_set",
                    "id": change_set_id,
                    "summary": format!("change set {} for transition {}", change_set.id.as_str(), change_set.transition_id),
                    "artifact": change_set.clone(),
                    "related": {
                        "run_id": change_set.run_id.clone(),
                        "assignment_id": change_set.assignment_id.as_ref().map(|id| id.as_str().to_string()),
                        "handoff_manifest_id": change_set.handoff_manifest_id.as_ref().map(|id| id.as_str().to_string()),
                        "validation_results": change_set.validation_results.clone(),
                        "synthetic": synthetic,
                        "synthetic_source": synthetic_source,
                    },
                    "next_commands": next_commands_for_change_set(&change_set),
                }),
            );
        }
        ChangeSetAction::List { run_id } => {
            let run_id = resolve_optional_run_id(store, run_id)?;
            let mut change_sets = Vec::new();
            for object in store.scan_objects()?.scanned_objects {
                if object.envelope.kind != Kind::ChangeSet {
                    continue;
                }
                let change_set: earmark_core::ChangeSet =
                    serde_json::from_slice(&object.payload.bytes)?;
                if let Some(rid) = &run_id {
                    if change_set.run_id != *rid {
                        continue;
                    }
                }
                change_sets.push(change_set);
            }
            emit(as_json, serde_json::to_value(change_sets)?);
        }
    }
    Ok(())
}

pub(crate) fn handle_handoff(
    ctx: &mut CommandContext,
    command: HandoffCommand,
) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    match command.action {
        HandoffAction::Show { handoff_id } => {
            let handoff = load_handoff_by_id(store, &handoff_id)?;
            emit(as_json, serde_json::to_value(handoff)?);
        }
        HandoffAction::Explain { handoff_id } => {
            let handoff = load_handoff_by_id(store, &handoff_id)?;
            emit(
                as_json,
                json!({
                    "kind": "handoff",
                    "id": handoff_id,
                    "summary": format!("handoff {} from transition {}", handoff.id.as_str(), handoff.from_transition_id),
                    "artifact": handoff.clone(),
                    "related": {
                        "run_id": handoff.run_id,
                        "source_change_set_id": handoff.source_change_set_id.as_str().to_string(),
                        "source_assignment_id": handoff.source_assignment_id.clone().map(|id| id.as_str().to_string()),
                        "to_transition_id": handoff.to_transition_id,
                        "allowed_input_classes": handoff.allowed_input_classes,
                        "allowed_output_classes": handoff.allowed_output_classes,
                        "allowed_relation_types": handoff.allowed_relation_types,
                        "standing_constraints": handoff.standing_constraints,
                        "required_checks": handoff.required_checks,
                    },
                    "next_commands": next_commands_for_handoff(&handoff),
                }),
            );
        }
        HandoffAction::List { run_id } => {
            let run_id = resolve_optional_run_id(store, run_id)?;
            let mut handoffs = Vec::new();
            for object in store.scan_objects()?.scanned_objects {
                if object.envelope.kind != Kind::HandoffManifest {
                    continue;
                }
                let handoff: earmark_core::HandoffManifest =
                    serde_json::from_slice(&object.payload.bytes)?;
                if let Some(rid) = &run_id {
                    if handoff.run_id != *rid {
                        continue;
                    }
                }
                handoffs.push(handoff);
            }
            emit(as_json, serde_json::to_value(handoffs)?);
        }
    }
    Ok(())
}

pub(crate) fn handle_failure(
    ctx: &mut CommandContext,
    command: FailureCommand,
) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    match command.action {
        FailureAction::Show { failure_id } => {
            let failure = load_failure_by_id(store, &failure_id)?;
            emit(as_json, serde_json::to_value(failure)?);
        }
        FailureAction::Explain { failure_id } => {
            let failure = load_failure_by_id(store, &failure_id)?;
            emit(
                as_json,
                json!({
                    "kind": "failure",
                    "id": failure_id,
                    "summary": format!("failure on transition {}", failure.transition_id),
                    "artifact": failure.clone(),
                    "related": {
                        "run_id": failure.run_id,
                        "assignment_id": failure.assignment_id.as_str().to_string(),
                        "failed_change_set_id": failure.failed_change_set_id.clone().map(|id| id.as_str().to_string()),
                        "error_type": failure.error_type,
                    },
                    "next_commands": next_commands_for_failure(&failure_id, &failure),
                }),
            );
        }
        FailureAction::List {
            run_id,
            transition_id,
        } => {
            let run_id = resolve_optional_run_id(store, run_id)?;
            let failures = list_failures(store, run_id.as_ref(), transition_id.as_deref())?;
            emit(
                as_json,
                json!({
                    "summary": format!("{} failures found", failures.len()),
                    "failures": failures,
                    "next_commands": ["em failure show <failure_id>", "em failure explain <failure_id>"],
                }),
            );
        }
    }
    Ok(())
}

pub(crate) fn handle_audit(
    ctx: &mut CommandContext,
    command: AuditCommand,
) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    match command.action {
        AuditAction::Failures {
            run_id,
            transition_id,
        } => {
            let run_id = resolve_optional_run_id(store, run_id)?;
            let mut failures = Vec::new();
            failures.extend(list_failures(
                store,
                run_id.as_ref(),
                transition_id.as_deref(),
            )?);
            emit(
                as_json,
                json!({
                    "kind": "audit_failures",
                    "id": "search",
                    "summary": format!("{} failures found", failures.len()),
                    "failures": failures,
                    "next_commands": [
                        "em failure show <failure_id>",
                        "em failure explain <failure_id>",
                    ],
                }),
            );
        }
        AuditAction::Show { failure_id } => {
            let failure = load_failure_by_id(store, &failure_id)?;
            emit(as_json, serde_json::to_value(failure)?);
        }
    }
    Ok(())
}

pub(crate) fn handle_report(
    ctx: &mut CommandContext,
    command: ReportCommand,
) -> Result<(), CliError> {
    let store = ctx.store;
    let index = &mut ctx.index;
    let as_json = ctx.as_json;

    match command.action {
        ReportAction::Run { target_id, output } => {
            let resolved_id = resolve_run_id(store, &target_id)?;
            let report = generate_run_report(store, &resolved_id)?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&output, report)?;
            emit(
                as_json,
                json!({
                    "kind": "report_generation",
                    "target_kind": "run",
                    "target_id": resolved_id,
                    "output": output.display().to_string(),
                }),
            );
        }
        ReportAction::Handoff { target_id, output } => {
            let report = generate_handoff_report(store, &target_id)?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&output, report)?;
            emit(
                as_json,
                json!({
                    "kind": "report_generation",
                    "target_kind": "handoff",
                    "target_id": target_id,
                    "output": output.display().to_string(),
                }),
            );
        }
        ReportAction::System { target_id, output } => {
            let report = generate_system_report(
                store,
                index
                    .as_ref()
                    .ok_or_else(|| CliError::argument("Index not available. This command must be run within an Earmark workspace."))?,
                &target_id,
            )?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&output, report)?;
            emit(
                as_json,
                json!({
                    "kind": "report_generation",
                    "target_kind": "system",
                    "target_id": target_id,
                    "output": output.display().to_string(),
                }),
            );
        }
    }
    Ok(())
}

pub(crate) fn handle_provider(
    ctx: &mut CommandContext,
    command: ProviderCommand,
) -> Result<(), CliError> {
    let as_json = ctx.as_json;

    match command.action {
        ProviderAction::Capabilities => {
            emit(
                as_json,
                json!({
                    "kind": "provider_capabilities",
                    "providers": ctx.provider_registry.capabilities(),
                    "loaded_provider_plugins": ctx.loaded_provider_plugins,
                }),
            );
        }
        ProviderAction::List => {
            let search_paths = crate::config::resolve_provider_plugin_dirs(ctx.root, ctx.config);
            emit(
                as_json,
                json!({
                    "kind": "provider_list",
                    "search_paths": search_paths,
                    "loaded_plugins": ctx.loaded_provider_plugins,
                }),
            );
        }
    }
    Ok(())
}

pub(crate) fn handle_status(ctx: &mut CommandContext) -> Result<(), CliError> {
    let store = ctx.store;
    let index = &mut ctx.index;
    let as_json = ctx.as_json;

    let (object_count, active_system_count) = require_index(index)?.counts()?;
    let assignments = list_assignments(store)?;
    let change_sets = list_change_sets(store)?;
    let handoffs = list_handoffs(store)?;
    let failures = list_failure_objects(store)?;
    let runs = list_run_records(store)?;
    let mut assignments_by_status: BTreeMap<String, usize> = BTreeMap::new();
    for assignment in assignments {
        let key = format!("{:?}", assignment.status).to_lowercase();
        *assignments_by_status.entry(key).or_insert(0) += 1;
    }
    let active_systems = require_index(index)?.get_active_systems()?;
    let latest_run = runs.last().map(|r| r.run_id.clone());

    emit(
        as_json,
        json!({
            "kind": "status",
            "id": "workspace",
            "summary": "workspace status",
            "object_count": object_count,
            "active_system_count": active_system_count,
            "active_systems": active_systems,
            "latest_run": latest_run,
            "assignment_count_by_status": assignments_by_status,
            "change_set_count": change_sets.len(),
            "handoff_count": handoffs.len(),
            "failure_count": failures.len(),
            "run_count": runs.len(),
            "metrics": crate::metrics::snapshot(),
            "provider_capabilities": ctx.provider_registry.capabilities(),
            "loaded_provider_plugins": ctx.loaded_provider_plugins,
            "root": store.root().display().to_string(),
            "paths": {
                "declarations_dir": store.declarations_dir().display().to_string(),
                "canonical_dir": store.root().join(".earmark").join("canonical").display().to_string(),
                "index_path": store.root().join(".earmark").join("derived").join("index.sqlite").display().to_string(),
            },
            "next_commands": ["em doctor", "em run list", "em audit failures"],
        }),
    );
    Ok(())
}

pub(crate) fn handle_relation(
    ctx: &mut CommandContext,
    command: RelationCommand,
) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    match command.action {
        RelationAction::Show { relation_id } => {
            let relation = load_relation_object_by_id(store, &relation_id)?;
            emit(as_json, serde_json::to_value(relation)?);
        }
        RelationAction::Explain { relation_id } => {
            let relation = load_relation_object_by_id(store, &relation_id)?;
            let payload: earmark_core::RelationPayload =
                serde_json::from_slice(&relation.payload.bytes)?;

            let mut related = json!({
                "source": payload.source,
                "target": payload.target,
                "relation_type": payload.relation_type,
            });

            if let Some(mode) = relation.envelope.headers.get("relation_creation_mode") {
                related["creation_mode"] = json!(mode);
            }

            let auth = {
                let endpoint = relation.envelope.headers.get("relation_auth_endpoint");
                let class_val = relation.envelope.headers.get("relation_auth_class");
                let authority = relation.envelope.headers.get("relation_auth_authority");
                let direction = relation.envelope.headers.get("relation_auth_direction");
                if endpoint.is_some()
                    || class_val.is_some()
                    || authority.is_some()
                    || direction.is_some()
                {
                    json!({
                        "endpoint": endpoint,
                        "class": class_val,
                        "authority": authority,
                        "direction": direction,
                    })
                } else {
                    serde_json::Value::Null
                }
            };
            related["authorization"] = auth;

            emit(
                as_json,
                json!({
                    "kind": "relation",
                    "id": relation_id,
                    "summary": format!("relation '{}' from {} to {}", payload.relation_type, payload.source.id, payload.target.id),
                    "artifact": relation.clone(),
                    "related": related,
                    "next_commands": [
                        format!("em relation show {}", relation_id),
                        format!("em query --object-id {}", payload.source.id),
                        format!("em query --object-id {}", payload.target.id),
                    ]
                }),
            );
        }
        RelationAction::List {
            source_id,
            target_id,
            relation_type,
        } => {
            let mut relations = Vec::new();
            for object in store.scan_objects()?.scanned_objects {
                if object.envelope.kind != Kind::Relation {
                    continue;
                }
                if let Some(head_ref) = store.read_head_ref(&object.envelope.id)? {
                    if head_ref.version_id != object.envelope.version_id {
                        continue;
                    }
                }
                let payload: earmark_core::RelationPayload =
                    serde_json::from_slice(&object.payload.bytes)?;
                if let Some(sid) = &source_id {
                    if payload.source.id.as_str() != sid {
                        continue;
                    }
                }
                if let Some(tid) = &target_id {
                    if payload.target.id.as_str() != tid {
                        continue;
                    }
                }
                if let Some(rt) = &relation_type {
                    if &payload.relation_type != rt {
                        continue;
                    }
                }
                relations.push(object);
            }
            emit(as_json, serde_json::to_value(relations)?);
        }
    }
    Ok(())
}

pub(crate) fn handle_standing_request(
    ctx: &mut CommandContext,
    command: StandingRequestCommand,
) -> Result<(), CliError> {
    let store = ctx.store;
    let index = &mut ctx.index;
    let as_json = ctx.as_json;

    match command.action {
        StandingRequestAction::List { status, target } => {
            let index = require_index(index)?;
            let mut requests = Vec::new();
            let objects = index.get_objects_by_kind(Kind::Object)?;
            for obj_ref in objects {
                let obj = store.read_version(&obj_ref)?;
                if obj.envelope.class.as_deref() == Some("standing_transition_request") {
                    let request: earmark_core::StandingTransitionRequest =
                        serde_json::from_slice(&obj.payload.bytes)?;
                    if let Some(status_str) = &status {
                        if format!("{:?}", request.status).to_lowercase()
                            != status_str.to_lowercase()
                        {
                            continue;
                        }
                    }
                    if let Some(target_id) = &target {
                        if request.target_object_id.as_str() != target_id {
                            continue;
                        }
                    }
                    requests.push(json!({
                        "id": obj.envelope.id.as_str(),
                        "version_id": obj.envelope.version_id.as_str(),
                        "request": request
                    }));
                }
            }
            emit(as_json, serde_json::to_value(requests)?);
        }
        StandingRequestAction::Show { request_id } => {
            let index = require_index(index)?;
            let id = earmark_core::ObjectId::parse(&request_id)
                .map_err(|e| CliError::argument(e.to_string()))?;
            let head_ref = index
                .get_head(&id)?
                .ok_or_else(|| CliError::not_found(format!("request {}", request_id)))?;
            let obj = store.read_version(&head_ref)?;
            let request: earmark_core::StandingTransitionRequest =
                serde_json::from_slice(&obj.payload.bytes)?;
            emit(
                as_json,
                json!({
                    "id": obj.envelope.id.as_str(),
                    "version_id": obj.envelope.version_id.as_str(),
                    "request": request
                }),
            );
        }
        StandingRequestAction::Approve { request_id, reason } => {
            let index = require_index_mut(index)?;
            let id = earmark_core::ObjectId::parse(&request_id)
                .map_err(|e| CliError::argument(e.to_string()))?;
            let head_ref = index
                .get_head(&id)?
                .ok_or_else(|| CliError::not_found(format!("request {}", request_id)))?;
            let new_version = earmark_exec::governance_ops::approve_standing_request(
                store, index, &head_ref, reason,
            )?;
            emit(
                as_json,
                json!({
                    "request_id": request_id,
                    "new_version_id": new_version.version_id.as_str(),
                    "status": "approved"
                }),
            );
        }
        StandingRequestAction::Reject { request_id, reason } => {
            let index = require_index_mut(index)?;
            let id = earmark_core::ObjectId::parse(&request_id)
                .map_err(|e| CliError::argument(e.to_string()))?;
            let head_ref = index
                .get_head(&id)?
                .ok_or_else(|| CliError::not_found(format!("request {}", request_id)))?;
            let new_version = earmark_exec::governance_ops::reject_standing_request(
                store, index, &head_ref, reason,
            )?;
            emit(
                as_json,
                json!({
                    "request_id": request_id,
                    "new_version_id": new_version.version_id.as_str(),
                    "status": "rejected"
                }),
            );
        }
        StandingRequestAction::Apply {
            request_id,
            policy,
            reason,
        } => {
            let index = require_index_mut(index)?;
            let id = earmark_core::ObjectId::parse(&request_id)
                .map_err(|e| CliError::argument(e.to_string()))?;
            let head_ref = index
                .get_head(&id)?
                .ok_or_else(|| CliError::not_found(format!("request {}", request_id)))?;

            let registry = earmark_core::StandingRegistry::kernel_defaults();
            let (target_ref, request_ref) = earmark_exec::governance_ops::apply_standing_request(
                store,
                index,
                &head_ref,
                policy.as_deref(),
                reason,
                &registry,
            )?;

            emit(
                as_json,
                json!({
                    "request_id": request_id,
                    "new_request_version_id": request_ref.version_id.as_str(),
                    "target_id": target_ref.id.as_str(),
                    "new_target_version_id": target_ref.version_id.as_str(),
                    "status": "applied"
                }),
            );
        }
    }
    Ok(())
}

fn require_index(index: &Option<DerivedIndex>) -> Result<&DerivedIndex, CliError> {
    index
        .as_ref()
        .ok_or_else(|| CliError::WorkspaceNotInitialized {
            status: WorkspaceLayoutStatus {
                root_exists: false,
                git_exists: false,
                manifest_exists: false,
                canonical_dir_exists: false,
                objects_dir_exists: false,
                payloads_dir_exists: false,
                heads_dir_exists: false,
                derived_dir_exists: false,
                work_surfaces_dir_exists: false,
                declarations_dir_exists: false,
                corpus_dir_exists: false,
            },
        })
}

fn require_index_mut(index: &mut Option<DerivedIndex>) -> Result<&mut DerivedIndex, CliError> {
    index
        .as_mut()
        .ok_or_else(|| CliError::WorkspaceNotInitialized {
            status: WorkspaceLayoutStatus {
                root_exists: false,
                git_exists: false,
                manifest_exists: false,
                canonical_dir_exists: false,
                objects_dir_exists: false,
                payloads_dir_exists: false,
                heads_dir_exists: false,
                derived_dir_exists: false,
                work_surfaces_dir_exists: false,
                declarations_dir_exists: false,
                corpus_dir_exists: false,
            },
        })
}
