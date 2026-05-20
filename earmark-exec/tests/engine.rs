#![allow(unused_imports)]

use std::collections::BTreeMap;

use chrono::Utc;
use earmark_core::{
    to_yaml, ChangeSet, ClassDefinition, ClassStandingRules, CompiledContextExpansion,
    CompiledContextRender, CompiledContextSelect, CompiledContextTemplate,
    CompiledContextVisibility, DimensionId, HandoffManifest, HeaderValue, InstructionPayload,
    JsonSchemaRef, Kind, MarkdownBody, Provenance, RuntimeProfile, Standing, SystemDefinition,
    TokenId, TransformationFailure, TransitionAssignment, VersionRef,
};
use earmark_exec::{ExecError, ExecutionEngine, ProviderRegistry, WorkflowRunRequest};
use earmark_index::DerivedIndex;
use earmark_store::{
    GitCanonicalStore, ObjectStore, StoreScanner, StoredObject, StoredPayload, WorkspaceLayout,
};
use tempfile::tempdir;

#[path = "engine/error_handling.rs"]
mod error_handling;
#[path = "engine/handoffs.rs"]
mod handoffs;
#[path = "engine/lifecycle.rs"]
mod lifecycle;
#[path = "engine/transitions.rs"]
mod transitions;

fn persist_transition_assignment(store: &GitCanonicalStore, assignment: &TransitionAssignment) {
    let stored = StoredObject::new(
        Kind::TransitionAssignment,
        Some("transition_assignment".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(assignment).unwrap()),
        vec![],
    );
    store.write_object(&stored).unwrap();
}

fn persist_handoff_manifest(store: &GitCanonicalStore, handoff: &HandoffManifest) {
    let stored = StoredObject::new(
        Kind::HandoffManifest,
        Some("handoff_manifest".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(handoff).unwrap()),
        vec![],
    );
    store.write_object(&stored).unwrap();
}

fn review_only_fixture(store: &GitCanonicalStore, class: &str) -> (VersionRef, VersionRef) {
    let workflow_yaml = format!(
        r#"name: review_flow
version: "1"
description: review bounded input
operations:
  - id: op_review
    kind: review
    input_contracts: [{class}]
    output_contracts: []
    instruction: null
    compiled_context: null
    policy: null
    provider_profile: null
edges: []
guards: []
"#
    );
    let workflow_obj = StoredObject::new(
        Kind::Workflow,
        Some("composition_workflow".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_yaml(workflow_yaml),
        vec![],
    );
    let workflow_ref = store.write_object(&workflow_obj).unwrap();
    let system = SystemDefinition {
        system_id: "test-system".to_string(),
        namespace: "systems/test".to_string(),
        title: "Test System".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![VersionRef::new(
            workflow_ref.id.clone(),
            workflow_ref.version_id.clone(),
        )],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![],
        runtime_profile: RuntimeProfile {
            execution_surface: "runtime_over_folder".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "materialized_manifest".to_string(),
        },
        activated_at: None,
    };
    let system_obj = StoredObject::new(
        Kind::SystemDefinition,
        Some("system_definition".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_yaml(to_yaml(&system).unwrap()),
        vec![],
    );
    let system_ref = store.write_object(&system_obj).unwrap();
    (
        VersionRef::new(system_ref.id, system_ref.version_id),
        VersionRef::new(workflow_ref.id, workflow_ref.version_id),
    )
}
