use earmark_connected_context::{WorkSurfaceManifest, WorkSurfaceObject};
use earmark_core::{
    DimensionId, Kind, ObjectId, ObjectRef, ProviderExposure, ProviderProfile, Standing,
    StandingRegistry, TokenId, VersionId, VersionRef,
};
use earmark_exec::helpers::render_provider_input;
use earmark_exec::{ProviderFailureKind, ProviderFailure};
use earmark_index::DerivedIndex;
use earmark_store::{GitCanonicalStore, ObjectStore, StoredObject, StoredPayload, WorkspaceLayout};
use std::collections::BTreeMap;
use tempfile::tempdir;

fn setup_env() -> (GitCanonicalStore, DerivedIndex) {
    let dir = tempdir().unwrap();
    let root = dir.keep();
    let store = GitCanonicalStore::new(root.clone());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(&root).unwrap();
    (store, index)
}

fn default_profile() -> ProviderProfile {
    ProviderProfile {
        name: "test".to_string(),
        version: "1".to_string(),
        description: None,
        provider: "mock".to_string(),
        model: "echo".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: earmark_core::ProviderBudget::default(),
        allowed_operations: vec!["transform".to_string()],
        exposure: ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: true,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: earmark_core::ProviderResponseContract {
            format: earmark_core::ProviderResponseFormat::Markdown,
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        http: None,
    }
}

fn instruction() -> earmark_core::InstructionPayload {
    earmark_core::InstructionPayload {
        name: "test".to_string(),
        version: "1".to_string(),
        purpose: "test".to_string(),
        input_classes: vec![],
        output_classes: vec![],
        execution_policy: "delegated".to_string(),
        provider_profile: None,
        trace_policy: "full".to_string(),
        register: "machined".to_string(),
        body: earmark_core::MarkdownBody::new("test instruction".to_string()),
    }
}

fn obj_with_payload(
    store: &GitCanonicalStore,
    kind: Kind,
    class: Option<&str>,
    standing: Standing,
    payload: &str,
) -> ObjectRef {
    let stored = StoredObject::new(
        kind.clone(),
        class.map(|s| s.to_string()),
        standing,
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("test".to_string()),
        )]),
        StoredPayload::from_markdown(payload),
        vec![],
    );
    let version = store.write_object(&stored).unwrap();
    ObjectRef::new(
        stored.envelope.id.clone(),
        version.version_id,
        kind.clone(),
        class.map(|s| s.to_string()),
    )
}

fn custom_visibility_registry() -> StandingRegistry {
    use earmark_core::ProtocolBinding;
    let sys = earmark_core::SystemDefinition {
        system_id: "sys_vis".to_string(),
        namespace: "systems/vis".to_string(),
        title: "Vis".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![earmark_core::StandingDimensionDefinition {
            id: DimensionId::from_static("dim:visibility"),
            default: TokenId::from_static("default"),
            tokens: vec![
                earmark_core::StandingTokenDefinition {
                    id: TokenId::from_static("default"),
                    implements: vec![],
                },
                earmark_core::StandingTokenDefinition {
                    id: TokenId::from_static("exposable"),
                    implements: vec![ProtocolBinding {
                        protocol: earmark_core::KernelProtocolId::from_static("kernel:visibility"),
                        state: None,
                        properties: BTreeMap::from([
                            (
                                "include_in_standard_context".to_string(),
                                earmark_core::ScalarValue::Bool(true),
                            ),
                            (
                                "expose_to_provider".to_string(),
                                earmark_core::ScalarValue::Bool(true),
                            ),
                        ]),
                    }],
                },
                earmark_core::StandingTokenDefinition {
                    id: TokenId::from_static("hidden_from_provider"),
                    implements: vec![ProtocolBinding {
                        protocol: earmark_core::KernelProtocolId::from_static("kernel:visibility"),
                        state: None,
                        properties: BTreeMap::from([(
                            "expose_to_provider".to_string(),
                            earmark_core::ScalarValue::Bool(false),
                        )]),
                    }],
                },
            ],
        }],
        runtime_profile: earmark_core::RuntimeProfile {
            execution_surface: "test".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "strict".to_string(),
        },
        activated_at: None,
    };
    StandingRegistry::from_system_definition(&sys).expect("vis registry")
}

fn standing_with(token: &str) -> Standing {
    let mut s = Standing::default();
    s.values.insert(
        DimensionId::from_static("dim:visibility"),
        TokenId::new(token),
    );
    s
}

/// Helper to create a work surface manifest with given object refs.
fn manifest_with(objects: Vec<ObjectRef>) -> WorkSurfaceManifest {
    WorkSurfaceManifest {
        surface_id: "test".to_string(),
        compiled_context: VersionRef::new(ObjectId::new(), VersionId::new()),
        work_packet: None,
        generated_at: chrono::Utc::now(),
        objects: objects
            .into_iter()
            .map(|o| WorkSurfaceObject {
                object: o,
                title: None,
                path: "test".to_string(),
                excerpt_range: None,
                lineage: vec![],
            })
            .collect(),
        boundary_relations: vec![],
        constraints: BTreeMap::new(),
        warnings: vec![],
    }
}

#[test]
fn test_default_standing_excludes_from_provider() {
    let (store, index) = setup_env();
    let payload = "VISIBLE_PAYLOAD";
    let obj_ref = obj_with_payload(
        &store,
        Kind::Object,
        Some("evidence"),
        Standing::default(),
        payload,
    );
    index.rebuild_from_store(&store).unwrap();

    let registry = StandingRegistry::kernel_defaults();
    let rendered = render_provider_input(
        &store,
        &instruction(),
        Some(&manifest_with(vec![obj_ref])),
        &[],
        &default_profile(),
        &registry,
    )
    .unwrap();

    // Default has expose_to_provider=false, so payload should be hidden
    assert!(
        !rendered.contains(payload),
        "default standing should not expose payload to provider"
    );
    assert!(
        !rendered.contains("Title:"),
        "default standing should not leak title to provider"
    );
    assert!(
        rendered.contains("Evidence item omitted from provider input"),
        "should contain generic omission notice"
    );
}

#[test]
fn test_standing_expose_to_provider_allows_payload() {
    let (store, index) = setup_env();
    let payload = "EXPOSABLE_PAYLOAD";
    let obj_ref = obj_with_payload(
        &store,
        Kind::Object,
        Some("evidence"),
        standing_with("exposable"),
        payload,
    );
    index.rebuild_from_store(&store).unwrap();

    let registry = custom_visibility_registry();
    let rendered = render_provider_input(
        &store,
        &instruction(),
        Some(&manifest_with(vec![obj_ref])),
        &[],
        &default_profile(),
        &registry,
    )
    .unwrap();

    assert!(
        rendered.contains(payload),
        "object with expose_to_provider=true should include payload"
    );
}

#[test]
fn test_standing_hides_from_provider_when_expose_false() {
    let (store, index) = setup_env();
    let payload = "HIDDEN_PROVIDER_PAYLOAD";
    let obj_ref = obj_with_payload(
        &store,
        Kind::Object,
        Some("evidence"),
        standing_with("hidden_from_provider"),
        payload,
    );
    index.rebuild_from_store(&store).unwrap();

    let registry = custom_visibility_registry();
    let rendered = render_provider_input(
        &store,
        &instruction(),
        Some(&manifest_with(vec![obj_ref])),
        &[],
        &default_profile(),
        &registry,
    )
    .unwrap();

    assert!(
        !rendered.contains(payload),
        "object with expose_to_provider=false should not include payload"
    );
    assert!(
        !rendered.contains("Title:"),
        "object with expose_to_provider=false should not leak title"
    );
    assert!(
        rendered.contains("Evidence item omitted from provider input"),
        "should contain generic omission notice"
    );
}

#[test]
fn test_two_gate_standing_permits_but_profile_denies() {
    let (store, index) = setup_env();
    let payload = "EXPOSABLE_BUT_PROFILE_DENIES";
    let obj_ref = obj_with_payload(
        &store,
        Kind::Object,
        Some("evidence"),
        standing_with("exposable"),
        payload,
    );
    index.rebuild_from_store(&store).unwrap();

    let mut profile = default_profile();
    profile.exposure.allow_prose_objects = false;

    let registry = custom_visibility_registry();
    let err = render_provider_input(
        &store,
        &instruction(),
        Some(&manifest_with(vec![obj_ref])),
        &[],
        &profile,
        &registry,
    )
    .unwrap_err();

    let provider_err = match &err {
        earmark_exec::ExecError::Provider(pf) => pf,
        _ => panic!("expected ExecError::Provider, got {:?}", err),
    };
    assert_eq!(provider_err.kind, ProviderFailureKind::PolicyViolation);
    assert!(provider_err.message.contains("allow_prose_objects is false"));
}

#[test]
fn test_two_gate_both_permit_payload_included() {
    let (store, index) = setup_env();
    let payload = "BOTH_GATES_PERMIT";
    let obj_ref = obj_with_payload(
        &store,
        Kind::Object,
        Some("evidence"),
        standing_with("exposable"),
        payload,
    );
    index.rebuild_from_store(&store).unwrap();

    let profile = default_profile(); // allow_prose_objects = true

    let registry = custom_visibility_registry();
    let rendered = render_provider_input(
        &store,
        &instruction(),
        Some(&manifest_with(vec![obj_ref])),
        &[],
        &profile,
        &registry,
    )
    .unwrap();

    // Both gates permit, payload should be included
    assert!(
        rendered.contains(payload),
        "both gates permit: payload should be included"
    );
}

#[test]
fn test_advisory_warning_does_not_leak_payload() {
    let (store, index) = setup_env();
    let secret_payload = "SECRET_PAYLOAD_SHOULD_NOT_LEAK";
    let obj_ref = obj_with_payload(
        &store,
        Kind::Object,
        Some("evidence"),
        standing_with("hidden_from_provider"),
        secret_payload,
    );
    index.rebuild_from_store(&store).unwrap();

    let registry = custom_visibility_registry();
    let rendered = render_provider_input(
        &store,
        &instruction(),
        Some(&manifest_with(vec![obj_ref])),
        &[],
        &default_profile(),
        &registry,
    )
    .unwrap();

    assert!(
        !rendered.contains(secret_payload),
        "advisory warning must not leak hidden payload"
    );
    assert!(
        !rendered.contains("Title:"),
        "advisory warning must not leak object title"
    );
    // Warning text must not contain the secret
    assert!(
        rendered.contains("Evidence item omitted from provider input"),
        "should contain generic omission notice"
    );
}

#[test]
fn test_expose_to_provider_true_still_blocked_by_structured_declaration_denial() {
    let (store, index) = setup_env();
    let body = "THIS_SHOULD_BE_HIDDEN_BY_STRUCTURED_DECLARATION_POLICY";
    let obj_payload = format!(
        "---\nname: test\nversion: \"1\"\npurpose: test\ninput_classes: []\noutput_classes: []\nexecution_policy: delegated\ntrace_policy: full\nregister: machined\n---\n{}",
        body
    );
    let obj_ref = obj_with_payload(
        &store,
        Kind::Instruction,
        None,
        standing_with("exposable"),
        &obj_payload,
    );
    index.rebuild_from_store(&store).unwrap();

    let mut profile = default_profile();
    profile.exposure.allow_structured_declarations = false;

    let registry = custom_visibility_registry();
    let err = render_provider_input(
        &store,
        &instruction(),
        Some(&manifest_with(vec![obj_ref])),
        &[],
        &profile,
        &registry,
    )
    .unwrap_err();

    let provider_err = match &err {
        earmark_exec::ExecError::Provider(pf) => pf,
        _ => panic!("expected ExecError::Provider, got {:?}", err),
    };
    assert_eq!(provider_err.kind, ProviderFailureKind::PolicyViolation);
    assert!(provider_err.message.contains("allow_structured_declarations is false"));
}
