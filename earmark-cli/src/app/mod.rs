use std::{collections::BTreeMap, fs, path::PathBuf};

pub(crate) mod common;
pub(crate) use common::CliError;
mod bootstrap;
mod commands;
mod dispatch;

use crate::cli::*;
use crate::output;
use clap_complete::{generate, shells};
use earmark_core::{
    FlexibleVersionRef, HeaderValue, Kind, ObjectRef, Provenance, Standing, VersionRef,
    WorkflowDeclaration, WorkflowDefinition, WorkflowOperation,
};
use earmark_declarations::{
    load_class_definition, load_compiled_context_template, load_instruction, load_provider_profile,
    load_standing_policy, load_system_definition, load_workflow_definition,
    validate_class_definition, validate_compiled_context_template, validate_instruction,
    validate_provider_profile, validate_standing_policy, validate_system_definition,
    validate_workflow_definition,
};
use earmark_index::DerivedIndex;
use earmark_store::{
    CanonicalStore, GitCanonicalStore, PayloadEncoding, StoredObject, StoredPayload,
};
use serde::Deserialize;
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

fn resolve_workflow_declaration(
    workflow_path: &std::path::Path,
    decl: WorkflowDeclaration,
    registry: &BTreeMap<std::path::PathBuf, VersionRef>,
) -> Result<WorkflowDefinition, CliError> {
    let mut operations = Vec::new();
    for op in decl.operations {
        operations.push(WorkflowOperation {
            id: op.id.clone(),
            kind: op.kind.clone(),
            input_contracts: op.input_contracts.clone(),
            output_contracts: op.output_contracts.clone(),
            instruction: resolve_flex_ref(workflow_path, op.instruction, registry)?,
            compiled_context: resolve_flex_ref(workflow_path, op.compiled_context, registry)?,
            policy: resolve_flex_ref(workflow_path, op.policy, registry)?,
            provider_profile: resolve_flex_ref(workflow_path, op.provider_profile, registry)?,
        });
    }

    Ok(WorkflowDefinition {
        name: decl.name,
        version: decl.version,
        description: decl.description,
        operations,
        edges: decl.edges,
        guards: decl.guards,
        output_contracts: decl.output_contracts,
    })
}

fn resolve_flex_ref(
    workflow_path: &std::path::Path,
    flex: Option<FlexibleVersionRef>,
    registry: &BTreeMap<std::path::PathBuf, VersionRef>,
) -> Result<Option<VersionRef>, CliError> {
    match flex {
        None => Ok(None),
        Some(FlexibleVersionRef::Ref(r)) => Ok(Some(r)),
        Some(FlexibleVersionRef::Path(p)) => {
            let rel_path = std::path::PathBuf::from(&p);
            let parent = workflow_path.parent().unwrap_or(workflow_path);
            let abs_path = parent.join(&rel_path);

            // Try to canonicalize for robust matching, but fall back to joined path
            let lookup_path = abs_path.canonicalize().unwrap_or_else(|_| abs_path.clone());

            if let Some(vref) = registry.get(&lookup_path) {
                Ok(Some(vref.clone()))
            } else {
                Err(CliError::argument(format!(
                    "unresolved path reference '{}' in workflow '{}'. Referenced declaration must be included in the system manifest.",
                    p,
                    workflow_path.display()
                )))
            }
        }
    }
}

impl DeclarationKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Instruction => "instruction",
            Self::StandingPolicy => "standing-policy",
            Self::Workflow => "workflow",
            Self::CompiledContext => "compiled-context",
            Self::ProviderProfile => "provider-profile",
            Self::System => "system",
        }
    }
}

fn validate_declaration_file<S: CanonicalStore>(
    store: &S,
    kind: DeclarationKind,
    path: &PathBuf,
) -> Result<serde_json::Value, CliError> {
    match kind {
        DeclarationKind::Class => {
            let declaration = load_class_definition(path)?;
            validate_class_definition(&declaration)?;
            Ok(json!({
                "name": declaration.name,
                "version": declaration.version,
                "object_kind": declaration.kind,
                "required_headers": declaration.required_headers,
                "relation_rule_count": declaration.relation_rules.len(),
            }))
        }
        DeclarationKind::Instruction => {
            let declaration = load_instruction(path)?;
            validate_instruction(&declaration)?;
            Ok(json!({
                "name": declaration.name,
                "version": declaration.version,
                "input_classes": declaration.input_classes,
                "output_classes": declaration.output_classes,
                "execution_policy": declaration.execution_policy,
                "trace_policy": declaration.trace_policy,
            }))
        }
        DeclarationKind::StandingPolicy => {
            let declaration = load_standing_policy(path)?;
            validate_standing_policy(&declaration)?;
            Ok(json!({
                "name": declaration.name,
                "version": declaration.version,
                "transition_rule_count": declaration.transition_rules.len(),
                "operation_requirement_count": declaration.operation_requirements.len(),
                "escalation_count": declaration.escalations.len(),
            }))
        }
        DeclarationKind::Workflow => {
            let declaration = load_workflow_definition(path).map_err(|e| {
                CliError::argument(format!(
                    "workflow parse/load error in {}: {}. Expected a workflow declaration with `operations`, `edges`, and `guards`.",
                    path.display(),
                    e
                ))
            })?;
            validate_workflow_definition(&declaration).map_err(|e| {
                CliError::argument(format!(
                    "workflow validation error in {}: {}. Repair workflow operation IDs and edge references so every `from`/`to` targets an existing operation.",
                    path.display(),
                    e
                ))
            })?;
            Ok(json!({
                "name": declaration.name,
                "version": declaration.version,
                "operation_count": declaration.operations.len(),
                "edge_count": declaration.edges.len(),
                "guard_count": declaration.guards.len(),
            }))
        }
        DeclarationKind::CompiledContext => {
            let declaration = load_compiled_context_template(path).map_err(|e| {
                CliError::argument(format!(
                    "compiled-context parse/load error in {}: {}. Expected a compiled-context template with `select` and `render` sections.",
                    path.display(),
                    e
                ))
            })?;
            validate_compiled_context_template(&declaration).map_err(|e| {
                CliError::argument(format!(
                    "compiled-context validation error in {}: {}. Provide a non-empty `name` and `render.mode`.",
                    path.display(),
                    e
                ))
            })?;
            Ok(json!({
                "name": declaration.name,
                "version": declaration.version,
                "selected_classes": declaration.select.classes,
                "selected_relations": declaration.select.relations,
                "render_mode": declaration.render.mode,
            }))
        }
        DeclarationKind::ProviderProfile => {
            let declaration = load_provider_profile(path).map_err(|e| {
                CliError::argument(format!(
                    "provider-profile parse/load error in {}: {}. Expected a provider-profile declaration with provider/model/response contract fields.",
                    path.display(),
                    e
                ))
            })?;
            validate_provider_profile(&declaration).map_err(|e| {
                CliError::argument(format!(
                    "provider-profile validation error in {}: {}. Provide non-empty `provider` and `model` values.",
                    path.display(),
                    e
                ))
            })?;
            Ok(json!({
                "name": declaration.name,
                "version": declaration.version,
                "provider": declaration.provider,
                "model": declaration.model,
                "allowed_operations": declaration.allowed_operations,
                "response_format": declaration.response_contract.format,
            }))
        }
        DeclarationKind::System => {
            if let Some(manifest) = try_load_path_system_manifest(path)? {
                validate_path_system_manifest(path, &manifest)?;
                Ok(json!({
                    "kind": "path_system_manifest",
                    "system_id": manifest.system_id,
                    "namespace": manifest.namespace,
                    "title": manifest.title,
                    "class_count": manifest.classes.len(),
                    "instruction_count": manifest.instructions.len(),
                    "policy_count": manifest.standing_policies.len(),
                    "workflow_count": manifest.workflows.len(),
                    "compiled_context_count": manifest.compiled_contexts.len(),
                    "provider_profile_count": manifest.provider_profiles.len(),
                }))
            } else {
                let declaration = load_system_definition(path)?;
                validate_system_definition(store, &declaration)?;
                Ok(json!({
                    "kind": "canonical_system_definition",
                    "system_id": declaration.system_id,
                    "namespace": declaration.namespace,
                    "title": declaration.title,
                    "class_count": declaration.classes.len(),
                    "instruction_count": declaration.instructions.len(),
                    "policy_count": declaration.policies.len(),
                    "workflow_count": declaration.workflows.len(),
                    "compiled_context_count": declaration.compiled_contexts.len(),
                    "provider_profile_count": declaration.provider_profiles.len(),
                }))
            }
        }
    }
}

fn explain_declaration_file<S: CanonicalStore>(
    store: &S,
    kind: DeclarationKind,
    path: &PathBuf,
) -> Result<serde_json::Value, CliError> {
    match kind {
        DeclarationKind::Class => {
            let declaration = load_class_definition(path)?;
            validate_class_definition(&declaration)?;
            Ok(json!({
                "title": format!("Class {}", declaration.name),
                "purpose": "Defines a canonical object class and its local validation rules.",
                "required_headers": declaration.required_headers,
                "payload_schema": declaration.payload_schema.0,
                "standing_rules": declaration.standing_rules,
                "allowed_relations": declaration.relation_rules.iter().map(|rule| json!({
                    "relation_type": rule.relation_type,
                    "counterparty_classes": rule.counterparty_classes,
                    "direction": rule.direction,
                    "authorizing_endpoint": rule.authorizing_endpoint,
                })).collect::<Vec<_>>(),
            }))
        }
        DeclarationKind::Instruction => {
            let declaration = load_instruction(path)?;
            validate_instruction(&declaration)?;
            Ok(json!({
                "title": format!("Instruction {}", declaration.name),
                "purpose": declaration.purpose,
                "accepts": declaration.input_classes,
                "emits": declaration.output_classes,
                "execution_policy": declaration.execution_policy,
                "trace_policy": declaration.trace_policy,
                "register": declaration.register,
                "body_preview": declaration.body.as_str().lines().take(3).collect::<Vec<_>>().join("\n"),
            }))
        }
        DeclarationKind::StandingPolicy => {
            let declaration = load_standing_policy(path)?;
            validate_standing_policy(&declaration)?;
            Ok(json!({
                "title": format!("Standing policy {}", declaration.name),
                "description": declaration.description,
                "transition_rules": declaration.transition_rules,
                "operation_requirements": declaration.operation_requirements,
                "escalations": declaration.escalations,
            }))
        }
        DeclarationKind::Workflow => {
            let declaration = load_workflow_definition(path)?;
            validate_workflow_definition(&declaration)?;
            let accepts = declaration
                .operations
                .iter()
                .flat_map(|operation| operation.input_contracts.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let emits = declaration
                .operations
                .iter()
                .flat_map(|operation| operation.output_contracts.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            Ok(json!({
                "title": format!("Workflow {}", declaration.name),
                "description": declaration.description,
                "accepts_input_classes": accepts,
                "produces_output_classes": emits,
                "operations": declaration.operations.iter().map(|operation| json!({
                    "id": operation.id,
                    "kind": operation.kind,
                    "input_contracts": operation.input_contracts,
                    "output_contracts": operation.output_contracts,
                    "has_instruction": operation.instruction.is_some(),
                    "has_compiled_context": operation.compiled_context.is_some(),
                    "has_policy": operation.policy.is_some(),
                    "has_provider_profile": operation.provider_profile.is_some(),
                    "standing_implications": operation.policy.as_ref().map(|policy| {
                        match policy {
                            FlexibleVersionRef::Ref(r) => vec![format!("policy-bound: {}@{}", r.id.as_str(), r.version_id.as_str())],
                            FlexibleVersionRef::Path(p) => vec![format!("policy-bound (path): {}", p)],
                        }
                    }).unwrap_or_default(),
                })).collect::<Vec<_>>(),
                "edges": declaration.edges,
                "guards": declaration.guards,
                "successor_handoff_behavior": declaration.edges.iter().map(|edge| json!({
                    "from_operation": edge.from,
                    "to_operation": edge.to,
                    "condition": edge.condition,
                })).collect::<Vec<_>>(),
            }))
        }
        DeclarationKind::CompiledContext => {
            let declaration = load_compiled_context_template(path)?;
            validate_compiled_context_template(&declaration)?;
            Ok(json!({
                "title": format!("Compiled Context {}", declaration.name),
                "description": declaration.description,
                "selects_classes": declaration.select.classes,
                "selects_relations": declaration.select.relations,
                "standing_filters": declaration.select.standing,
                "expansion": declaration.select.expansion,
                "bounded_depth_behavior": declaration.select.time_range,
                "inclusion_rationale": "Class and standing filters apply to seed objects and expansion by default; relation filters control traversable edges. Set expansion.object_filter=none to deliberately widen boundaries.",
                "render": declaration.render,
                "visibility": declaration.visibility,
            }))
        }
        DeclarationKind::ProviderProfile => {
            let declaration = load_provider_profile(path)?;
            validate_provider_profile(&declaration)?;
            Ok(json!({
                "title": format!("Provider Profile {}", declaration.name),
                "description": declaration.description,
                "provider": declaration.provider,
                "model": declaration.model,
                "budget": declaration.budget,
                "allowed_operations": declaration.allowed_operations,
                "exposure": declaration.exposure,
                "response_contract": declaration.response_contract,
            }))
        }
        DeclarationKind::System => {
            if let Some(manifest) = try_load_path_system_manifest(path)? {
                validate_path_system_manifest(path, &manifest)?;
                let activation_readiness = !manifest.workflows.is_empty()
                    && !manifest.compiled_contexts.is_empty()
                    && !manifest.provider_profiles.is_empty();
                Ok(json!({
                    "title": manifest.title,
                    "system_id": manifest.system_id,
                    "namespace": manifest.namespace,
                    "description": manifest.description,
                    "kind": "path_system_manifest",
                    "declaration_counts": {
                        "classes": manifest.classes.len(),
                        "instructions": manifest.instructions.len(),
                        "standing_policies": manifest.standing_policies.len(),
                        "workflows": manifest.workflows.len(),
                        "compiled_contexts": manifest.compiled_contexts.len(),
                        "provider_profiles": manifest.provider_profiles.len(),
                    },
                    "declaration_files_by_role": {
                        "classes": manifest.classes.clone(),
                        "instructions": manifest.instructions.clone(),
                        "standing_policies": manifest.standing_policies.clone(),
                        "workflows": manifest.workflows.clone(),
                        "compiled_contexts": manifest.compiled_contexts.clone(),
                        "provider_profiles": manifest.provider_profiles.clone(),
                    },
                    "workflow_inventory": manifest.workflows.clone(),
                    "compiled_context_inventory": manifest.compiled_contexts.clone(),
                    "provider_profile_inventory": manifest.provider_profiles.clone(),
                    "activation_readiness": activation_readiness,
                    "runtime_profile": manifest.runtime_profile,
                }))
            } else {
                let declaration = load_system_definition(path)?;
                validate_system_definition(store, &declaration)?;
                Ok(json!({
                    "title": declaration.title,
                    "system_id": declaration.system_id,
                    "namespace": declaration.namespace,
                    "description": declaration.description,
                    "kind": "canonical_system_definition",
                    "declaration_counts": {
                        "classes": declaration.classes.len(),
                        "instructions": declaration.instructions.len(),
                        "policies": declaration.policies.len(),
                        "workflows": declaration.workflows.len(),
                        "compiled_contexts": declaration.compiled_contexts.len(),
                        "provider_profiles": declaration.provider_profiles.len(),
                    },
                    "runtime_profile": declaration.runtime_profile,
                }))
            }
        }
    }
}

pub(crate) fn register_declaration_file<S: CanonicalStore>(
    store: &S,
    index: Option<&DerivedIndex>,
    kind: DeclarationKind,
    path: &PathBuf,
    registry: Option<&BTreeMap<PathBuf, VersionRef>>,
) -> Result<VersionRef, CliError> {
    let (stored_kind, name, payload, headers, explicit_symbolic_name) = match kind {
        DeclarationKind::Class => {
            let decl = load_class_definition(path)?;
            validate_class_definition(&decl)?;
            let mut headers = BTreeMap::new();
            headers.insert("title".to_string(), HeaderValue::String(decl.name.clone()));
            (
                Kind::Object,
                Some("class_definition".to_string()),
                StoredPayload::from_yaml(earmark_core::to_yaml(&decl)?),
                headers,
                decl.name.clone(),
            )
        }
        DeclarationKind::Instruction => {
            let decl = load_instruction(path)?;
            validate_instruction(&decl)?;
            let mut headers = BTreeMap::new();
            headers.insert("title".to_string(), HeaderValue::String(decl.name.clone()));
            (
                Kind::Instruction,
                Some(decl.name.clone()),
                StoredPayload::from_markdown(decl.to_markdown()?),
                headers,
                decl.name.clone(),
            )
        }
        DeclarationKind::StandingPolicy => {
            let decl = load_standing_policy(path)?;
            validate_standing_policy(&decl)?;
            let mut headers = BTreeMap::new();
            headers.insert("title".to_string(), HeaderValue::String(decl.name.clone()));
            (
                Kind::Policy,
                Some(decl.name.clone()),
                StoredPayload::from_yaml(earmark_core::to_yaml(&decl)?),
                headers,
                decl.name.clone(),
            )
        }
        DeclarationKind::Workflow => {
            let decl = load_workflow_definition(path)?;
            validate_workflow_definition(&decl)?;

            // Standalone registration must not contain path references
            for op in &decl.operations {
                let has_paths = [
                    &op.instruction,
                    &op.compiled_context,
                    &op.policy,
                    &op.provider_profile,
                ]
                .iter()
                .any(|opt| matches!(opt, Some(FlexibleVersionRef::Path(_))));

                if has_paths {
                    return Err(CliError::argument(format!(
                        "workflow path references require system-manifest registration (found in workflow '{}')",
                        decl.name
                    )));
                }
            }

            // Resolve into strict WorkflowDefinition
            let resolved =
                resolve_workflow_declaration(path, decl, registry.unwrap_or(&BTreeMap::new()))?;

            let mut headers = BTreeMap::new();
            headers.insert(
                "title".to_string(),
                HeaderValue::String(resolved.name.clone()),
            );
            (
                Kind::Workflow,
                Some(resolved.name.clone()),
                StoredPayload::from_yaml(earmark_core::to_yaml(&resolved)?),
                headers,
                resolved.name.clone(),
            )
        }
        DeclarationKind::CompiledContext => {
            let decl = load_compiled_context_template(path)?;
            validate_compiled_context_template(&decl)?;
            let mut headers = BTreeMap::new();
            headers.insert("title".to_string(), HeaderValue::String(decl.name.clone()));
            (
                Kind::CompiledContextTemplate,
                Some(decl.name.clone()),
                StoredPayload::from_yaml(earmark_core::to_yaml(&decl)?),
                headers,
                decl.name.clone(),
            )
        }
        DeclarationKind::ProviderProfile => {
            let decl = load_provider_profile(path)?;
            validate_provider_profile(&decl)?;
            let mut headers = BTreeMap::new();
            headers.insert("title".to_string(), HeaderValue::String(decl.name.clone()));
            (
                Kind::ProviderProfile,
                Some(decl.name.clone()),
                StoredPayload::from_yaml(earmark_core::to_yaml(&decl)?),
                headers,
                decl.name.clone(),
            )
        }
        DeclarationKind::System => {
            if let Some(manifest) = try_load_path_system_manifest(path)? {
                validate_path_system_manifest(path, &manifest)?;
                let decl = assemble_system_definition_from_manifest(store, path, &manifest)?;
                let mut headers = BTreeMap::new();
                headers.insert("title".to_string(), HeaderValue::String(decl.title.clone()));
                (
                    Kind::SystemDefinition,
                    Some("system_definition".to_string()),
                    StoredPayload::from_yaml(earmark_core::to_yaml(&decl)?),
                    headers,
                    decl.system_id.clone(),
                )
            } else {
                let decl = load_system_definition(path)?;
                validate_system_definition(store, &decl)?;
                let mut headers = BTreeMap::new();
                headers.insert("title".to_string(), HeaderValue::String(decl.title.clone()));
                (
                    Kind::SystemDefinition,
                    Some("system_definition".to_string()),
                    StoredPayload::from_yaml(earmark_core::to_yaml(&decl)?),
                    headers,
                    decl.system_id.clone(),
                )
            }
        }
    };

    let object = StoredObject::new(
        stored_kind,
        name,
        Standing::default(),
        Provenance::direct_input("operator"),
        headers,
        payload,
        vec![],
    );

    // Validate symbolic declaration names explicitly; durable ids are store-generated.
    earmark_core::SymbolicName::parse(explicit_symbolic_name)?;

    let version_ref = if let Some(idx) = index {
        earmark_exec::persistence_helpers::write_object_and_index(store, idx, &object)?
    } else {
        store.write_object(&object)?
    };
    Ok(version_ref)
}

fn resolve_version_ref<S: CanonicalStore>(
    store: &S,
    object_id: &str,
    version_id: Option<&str>,
) -> Result<VersionRef, CliError> {
    if let Some(version_id) = version_id {
        return Ok(VersionRef::new(
            earmark_core::ObjectId::parse(object_id.to_string())?,
            earmark_core::VersionId::parse(version_id.to_string())?,
        ));
    }
    store
        .read_head_ref(&earmark_core::ObjectId::parse(object_id.to_string())?)?
        .ok_or_else(|| CliError::not_found(format!("object head not found: {}", object_id)))
}

fn resolve_workflow_version_ref<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    workflow_id: &str,
    version_id: Option<&str>,
) -> Result<VersionRef, CliError> {
    if earmark_core::ObjectId::parse(workflow_id.to_string()).is_ok() {
        return resolve_version_ref(store, workflow_id, version_id);
    }
    if version_id.is_some() {
        return Err(CliError::argument(
            "workflow --version-id requires a durable workflow object id".to_string(),
        ));
    }
    index
        .resolve_workflow_symbolic_latest(workflow_id)?
        .ok_or_else(|| CliError::not_found(format!("workflow not found: {}", workflow_id)))
}

#[derive(Debug, Clone, Deserialize)]
struct PathSystemManifest {
    system_id: String,
    namespace: String,
    title: String,
    description: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    instructions: Vec<String>,
    #[serde(default)]
    standing_policies: Vec<String>,
    #[serde(default)]
    compiled_contexts: Vec<String>,
    #[serde(default)]
    provider_profiles: Vec<String>,
    #[serde(default)]
    workflows: Vec<String>,
    default_compiled_context: Option<String>,
    default_provider_profile: Option<String>,
    runtime_profile: earmark_core::RuntimeProfile,
}

fn try_load_path_system_manifest(
    path: &std::path::Path,
) -> Result<Option<PathSystemManifest>, CliError> {
    let text = fs::read_to_string(path)?;
    let value: serde_yaml::Value = serde_yaml::from_str(&text)?;
    let Some(classes) = value.get("classes").and_then(|v| v.as_sequence()) else {
        return Ok(None);
    };
    if !classes.iter().all(|entry| {
        entry
            .as_str()
            .map(|s| s.ends_with(".yaml") || s.ends_with(".md"))
            .unwrap_or(false)
    }) {
        return Ok(None);
    }
    serde_yaml::from_str(&text).map(Some).map_err(|error| {
        CliError::argument(format!(
            "system manifest parse error in {}: {}",
            path.display(),
            error
        ))
    })
}

fn resolve_manifest_path(manifest_path: &std::path::Path, rel: &str) -> PathBuf {
    let base = manifest_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    base.join(rel)
}

fn validate_path_system_manifest(
    path: &std::path::Path,
    manifest: &PathSystemManifest,
) -> Result<(), common::CliError> {
    for (role, refs) in [
        ("class", &manifest.classes),
        ("instruction", &manifest.instructions),
        ("standing-policy", &manifest.standing_policies),
        ("compiled-context", &manifest.compiled_contexts),
        ("provider-profile", &manifest.provider_profiles),
        ("workflow", &manifest.workflows),
    ] {
        for rel in refs {
            let p = resolve_manifest_path(path, rel);
            let result = match role {
                "class" => load_class_definition(&p)
                    .and_then(|d| validate_class_definition(&d).map(|_| d))
                    .map(|_| ()),
                "instruction" => load_instruction(&p)
                    .and_then(|d| validate_instruction(&d).map(|_| d))
                    .map(|_| ()),
                "standing-policy" => load_standing_policy(&p)
                    .and_then(|d| validate_standing_policy(&d).map(|_| d))
                    .map(|_| ()),
                "compiled-context" => load_compiled_context_template(&p)
                    .and_then(|d| validate_compiled_context_template(&d).map(|_| d))
                    .map(|_| ()),
                "provider-profile" => load_provider_profile(&p)
                    .and_then(|d| validate_provider_profile(&d).map(|_| d))
                    .map(|_| ()),
                "workflow" => load_workflow_definition(&p)
                    .and_then(|d| validate_workflow_definition(&d).map(|_| d))
                    .map(|_| ()),
                _ => Ok(()),
            };
            if let Err(error) = result {
                return Err(CliError::argument(format!(
                    "system manifest validation error in {}: `{}` is listed under `{}` but failed {} validation: {}. Repair the file or move it under the correct declaration role.",
                    path.display(),
                    rel,
                    role,
                    role,
                    error
                )));
            }
        }
    }
    Ok(())
}

fn assemble_system_definition_from_manifest<S: CanonicalStore>(
    store: &S,
    path: &std::path::Path,
    manifest: &PathSystemManifest,
) -> Result<earmark_core::SystemDefinition, CliError> {
    let mut registry = BTreeMap::new();

    let mut classes = Vec::new();
    for rel in &manifest.classes {
        let p = resolve_manifest_path(path, rel);
        let vref = register_declaration_file(store, None, DeclarationKind::Class, &p, None)?;
        classes.push(vref.clone());
        if let Ok(abs) = p.canonicalize() {
            registry.insert(abs, vref);
        } else {
            registry.insert(p, vref);
        }
    }
    let mut instructions = Vec::new();
    for rel in &manifest.instructions {
        let p = resolve_manifest_path(path, rel);
        let vref = register_declaration_file(store, None, DeclarationKind::Instruction, &p, None)?;
        instructions.push(vref.clone());
        if let Ok(abs) = p.canonicalize() {
            registry.insert(abs, vref);
        } else {
            registry.insert(p, vref);
        }
    }
    let mut policies = Vec::new();
    for rel in &manifest.standing_policies {
        let p = resolve_manifest_path(path, rel);
        let vref =
            register_declaration_file(store, None, DeclarationKind::StandingPolicy, &p, None)?;
        policies.push(vref.clone());
        if let Ok(abs) = p.canonicalize() {
            registry.insert(abs, vref);
        } else {
            registry.insert(p, vref);
        }
    }
    let mut compiled_contexts = Vec::new();
    for rel in &manifest.compiled_contexts {
        let p = resolve_manifest_path(path, rel);
        let vref =
            register_declaration_file(store, None, DeclarationKind::CompiledContext, &p, None)?;
        compiled_contexts.push(vref.clone());
        if let Ok(abs) = p.canonicalize() {
            registry.insert(abs, vref);
        } else {
            registry.insert(p, vref);
        }
    }
    let mut provider_profiles = Vec::new();
    for rel in &manifest.provider_profiles {
        let p = resolve_manifest_path(path, rel);
        let vref =
            register_declaration_file(store, None, DeclarationKind::ProviderProfile, &p, None)?;
        provider_profiles.push(vref.clone());
        if let Ok(abs) = p.canonicalize() {
            registry.insert(abs, vref);
        } else {
            registry.insert(p, vref);
        }
    }

    let workflows = manifest
        .workflows
        .iter()
        .map(|rel| {
            register_declaration_file(
                store,
                None,
                DeclarationKind::Workflow,
                &resolve_manifest_path(path, rel),
                Some(&registry),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let default_compiled_context = manifest
        .default_compiled_context
        .as_ref()
        .map(|rel| {
            register_declaration_file(
                store,
                None,
                DeclarationKind::CompiledContext,
                &resolve_manifest_path(path, rel),
                None,
            )
        })
        .transpose()?;
    let default_provider_profile = manifest
        .default_provider_profile
        .as_ref()
        .map(|rel| {
            register_declaration_file(
                store,
                None,
                DeclarationKind::ProviderProfile,
                &resolve_manifest_path(path, rel),
                None,
            )
        })
        .transpose()?;

    Ok(earmark_core::SystemDefinition {
        system_id: manifest.system_id.clone(),
        namespace: manifest.namespace.clone(),
        title: manifest.title.clone(),
        description: manifest.description.clone(),
        classes,
        instructions,
        policies,
        workflows,
        compiled_contexts,
        provider_profiles,
        default_compiled_context,
        default_provider_profile,
        standing_dimensions: vec![],
        runtime_profile: manifest.runtime_profile.clone(),
        activated_at: None,
    })
}

fn template_file_for_kind(kind: DeclarationKind) -> &'static str {
    match kind {
        DeclarationKind::Class => "templates/classes/class.yaml",
        DeclarationKind::Instruction => "templates/instructions/instruction.md",
        DeclarationKind::StandingPolicy => "templates/standing_policies/standing_policy.yaml",
        DeclarationKind::CompiledContext => "templates/compiled_contexts/compiled_context.yaml",
        DeclarationKind::ProviderProfile => "templates/provider_profiles/provider_profile.yaml",
        DeclarationKind::Workflow => "templates/workflows/workflow.yaml",
        DeclarationKind::System => "templates/systems/system_path_manifest.yaml",
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
    let template_path = root.join(template_file_for_kind(kind));
    let mut body = fs::read_to_string(&template_path)?;
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
    for object in store.scan_objects()? {
        if object.envelope.kind != Kind::RunRecord {
            continue;
        }
        let ledger: earmark_core::RunRecord = serde_json::from_slice(&object.payload.bytes)?;
        ledgers.push(ledger);
    }
    ledgers.sort_by_key(|ledger| ledger.started_at);
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
    for object in store.scan_objects()? {
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
    for object in store.scan_objects()? {
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
    for object in store.scan_objects()? {
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
    for object in store.scan_objects()? {
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
    for object in store.scan_objects()? {
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
    for object in store.scan_objects()? {
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
    for object in store.scan_objects()? {
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
    for object in store.scan_objects()? {
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
    let id = value.get("id")?.as_str().unwrap_or("unknown");
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
