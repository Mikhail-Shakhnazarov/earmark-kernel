use std::{collections::BTreeMap, fs, path::PathBuf};

use crate::app::common::CliError;
use crate::cli::DeclarationKind;
use earmark_core::{FlexibleVersionRef, HeaderValue, Kind, Provenance, Standing, VersionRef};
use earmark_declarations::{
    load_class_definition, load_compiled_context_template, load_instruction, load_provider_profile,
    load_standing_policy, load_system_definition, load_workflow_definition,
    resolve_instruction_declaration, resolve_workflow_declaration, validate_class_definition,
    validate_compiled_context_template, validate_instruction, validate_provider_profile,
    validate_standing_policy, validate_system_definition, validate_workflow_definition,
};
use earmark_index::DerivedIndex;
use earmark_store::{CanonicalStore, StoredObject, StoredPayload};
use serde::Deserialize;
use serde_json::json;

impl DeclarationKind {
    pub(crate) fn as_str(self) -> &'static str {
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

pub(crate) fn validate_declaration_file<S: CanonicalStore>(
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

pub(crate) fn explain_declaration_file<S: CanonicalStore>(
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
    index: Option<&mut DerivedIndex>,
    kind: DeclarationKind,
    path: &PathBuf,
    registry: Option<&BTreeMap<PathBuf, VersionRef>>,
    actor: &str,
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

            let resolved = if let Some(reg) = registry {
                resolve_instruction_declaration(path, decl, reg)?
            } else {
                if matches!(decl.provider_profile, Some(FlexibleVersionRef::Path(_))) {
                    return Err(CliError::argument(format!(
                        "instruction path references require system-manifest registration (found in instruction '{}')",
                        decl.name
                    )));
                }
                decl
            };

            let mut headers = BTreeMap::new();
            headers.insert(
                "title".to_string(),
                HeaderValue::String(resolved.name.clone()),
            );
            (
                Kind::Instruction,
                Some(resolved.name.clone()),
                StoredPayload::from_markdown(resolved.to_markdown()?),
                headers,
                resolved.name.clone(),
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

            for op in &decl.operations {
                let has_paths = [
                    &op.instruction,
                    &op.compiled_context,
                    &op.policy,
                    &op.provider_profile,
                ]
                .iter()
                .any(|opt| matches!(opt, Some(FlexibleVersionRef::Path(_))));

                if has_paths && registry.is_none() {
                    return Err(CliError::argument(format!(
                        "workflow path references require system-manifest registration (found in workflow '{}')",
                        decl.name
                    )));
                }
            }

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
                let decl = assemble_system_definition_from_manifest(store, path, &manifest, actor)?;
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
        Provenance::direct_input(actor),
        headers,
        payload,
        vec![],
    );

    earmark_core::SymbolicName::parse(explicit_symbolic_name)?;

    let version_ref = if let Some(idx) = index {
        earmark_exec::persistence_helpers::write_object_and_index(store, idx, &object)?
    } else {
        store.write_object(&object)?
    };
    Ok(version_ref)
}

pub(crate) fn resolve_version_ref<S: CanonicalStore>(
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

pub(crate) fn resolve_workflow_version_ref<S: CanonicalStore>(
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
    #[serde(default)]
    #[allow(dead_code)]
    pub schema: Option<String>,
    pub system_id: String,
    pub namespace: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(default)]
    pub classes: Vec<String>,
    #[serde(default)]
    pub instructions: Vec<String>,
    #[serde(default)]
    pub standing_policies: Vec<String>,
    #[serde(default)]
    pub compiled_contexts: Vec<String>,
    #[serde(default)]
    pub provider_profiles: Vec<String>,
    #[serde(default)]
    pub workflows: Vec<String>,
    pub default_compiled_context: Option<String>,
    pub default_provider_profile: Option<String>,
    pub runtime_profile: earmark_core::RuntimeProfile,
}

fn try_load_path_system_manifest(
    path: &std::path::Path,
) -> Result<Option<PathSystemManifest>, CliError> {
    let text = fs::read_to_string(path)?;
    let value: serde_yaml::Value = serde_yaml::from_str(&text)?;

    let schema = value.get("schema").and_then(|v| v.as_str());
    if schema != Some("earmark.path_system_manifest.v1") {
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
) -> Result<(), CliError> {
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
    actor: &str,
) -> Result<earmark_core::SystemDefinition, CliError> {
    let mut registry = BTreeMap::new();

    let mut classes = Vec::new();
    for rel in &manifest.classes {
        let p = resolve_manifest_path(path, rel);
        let vref = register_declaration_file(store, None, DeclarationKind::Class, &p, None, actor)?;
        classes.push(vref.clone());
        if let Ok(abs) = p.canonicalize() {
            registry.insert(abs, vref);
        } else {
            registry.insert(p, vref);
        }
    }
    let mut provider_profiles = Vec::new();
    for rel in &manifest.provider_profiles {
        let p = resolve_manifest_path(path, rel);
        let vref = register_declaration_file(
            store,
            None,
            DeclarationKind::ProviderProfile,
            &p,
            Some(&registry),
            actor,
        )?;
        provider_profiles.push(vref.clone());
        if let Ok(abs) = p.canonicalize() {
            registry.insert(abs, vref);
        } else {
            registry.insert(p, vref);
        }
    }

    let mut instructions = Vec::new();
    for rel in &manifest.instructions {
        let p = resolve_manifest_path(path, rel);
        let vref = register_declaration_file(
            store,
            None,
            DeclarationKind::Instruction,
            &p,
            Some(&registry),
            actor,
        )?;
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
        let vref = register_declaration_file(
            store,
            None,
            DeclarationKind::StandingPolicy,
            &p,
            Some(&registry),
            actor,
        )?;
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
        let vref = register_declaration_file(
            store,
            None,
            DeclarationKind::CompiledContext,
            &p,
            Some(&registry),
            actor,
        )?;
        compiled_contexts.push(vref.clone());
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
                actor,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let default_compiled_context = manifest
        .default_compiled_context
        .as_ref()
        .map(|rel| {
            let p = resolve_manifest_path(path, rel);
            let lookup_path = p.canonicalize().unwrap_or(p);
            registry.get(&lookup_path).cloned().ok_or_else(|| {
                CliError::argument(format!(
                    "default_compiled_context '{}' must be listed in the 'compiled_contexts' section of the manifest",
                    rel
                ))
            })
        })
        .transpose()?;

    let default_provider_profile = manifest
        .default_provider_profile
        .as_ref()
        .map(|rel| {
            let p = resolve_manifest_path(path, rel);
            let lookup_path = p.canonicalize().unwrap_or(p);
            registry.get(&lookup_path).cloned().ok_or_else(|| {
                CliError::argument(format!(
                    "default_provider_profile '{}' must be listed in the 'provider_profiles' section of the manifest",
                    rel
                ))
            })
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
