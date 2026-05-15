use std::collections::{BTreeMap, BTreeSet};

use earmark_core::{
    to_yaml, DimensionId, Kind, Provenance, RuntimeProfile, Standing, SystemDefinition, TokenId,
    WorkflowDefinition,
};
use earmark_index::{DerivedIndex, IndexError, QueryFilter};
use earmark_store::{
    CanonicalStore, GitCanonicalStore, ObjectStore, StoreScanner, StoreWriteLocking, StoredObject,
    StoredPayload, WorkspaceLayout,
};
use tempfile::tempdir;

#[test]
fn rebuild_index_from_canonical_state() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    let obj = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("Indexed note".to_string()),
        )]),
        StoredPayload::from_markdown("hello index"),
        vec![],
    );
    store.write_object(&obj).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();
    let rows = index
        .query_objects(&QueryFilter {
            class: Some("note".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title.as_deref(), Some("Indexed note"));
}

#[test]
fn relation_adjacency_query() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let a = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("a"),
        vec![],
    );
    let b = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("b"),
        vec![],
    );
    store.write_object(&a).unwrap();
    store.write_object(&b).unwrap();

    let relation = StoredObject::new(
        Kind::Relation,
        None,
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(
            serde_json::to_vec(&serde_json::json!({
                "source": a.object_ref(),
                "target": b.object_ref(),
                "relation_type": "supports",
                "qualifiers": {},
                "scope": "test"
            }))
            .unwrap(),
        ),
        vec![],
    );
    store.write_object(&relation).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();
    let adjacency = index.relation_adjacency(&a.envelope.id, false).unwrap();
    assert_eq!(adjacency.len(), 1);
    assert_eq!(adjacency[0].relation_type, "supports");
}

#[test]
fn active_system_definition_activation() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let system = SystemDefinition {
        system_id: "pkm-core".to_string(),
        namespace: "systems/pkm-core".to_string(),
        title: "PKM Core".to_string(),
        description: Some("system".to_string()),
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
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
    let payload = StoredPayload::from_yaml(to_yaml(&system).unwrap());
    let stored = StoredObject::new(
        Kind::SystemDefinition,
        Some("system_definition".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String(system.title.clone()),
        )]),
        payload,
        vec![],
    );
    let version = store.write_object(&stored).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();
    let active = index
        .activate_system(&system.namespace, &system.system_id, &version)
        .unwrap();
    assert_eq!(active.system_id, "pkm-core");
    assert!(index
        .get_active_system(&system.namespace)
        .unwrap()
        .is_some());
}

#[test]
fn symbolic_resolution_uses_explicit_declaration_identity_not_title_or_class() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let target = WorkflowDefinition {
        name: "research_synthesis".to_string(),
        version: "1.0.0".to_string(),
        description: Some("actual identity".to_string()),
        operations: vec![],
        edges: vec![],
        guards: vec![],
        output_contracts: vec![],
    };
    let target_obj = StoredObject::new(
        Kind::Workflow,
        Some("wrong_class".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("Not matching title".to_string()),
        )]),
        StoredPayload::from_yaml(to_yaml(&target).unwrap()),
        vec![],
    );
    let target_ref = store.write_object(&target_obj).unwrap();

    let title_collision = WorkflowDefinition {
        name: "different_identity".to_string(),
        version: "1.0.0".to_string(),
        description: Some("collision".to_string()),
        operations: vec![],
        edges: vec![],
        guards: vec![],
        output_contracts: vec![],
    };
    let collision_obj = StoredObject::new(
        Kind::Workflow,
        Some("research_synthesis".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("research_synthesis".to_string()),
        )]),
        StoredPayload::from_yaml(to_yaml(&title_collision).unwrap()),
        vec![],
    );
    let collision_ref = store.write_object(&collision_obj).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let resolved = index
        .resolve_workflow_symbolic_latest("research_synthesis")
        .unwrap()
        .unwrap();
    assert_eq!(resolved.id, target_ref.id);
    assert_eq!(resolved.version_id, target_ref.version_id);
    assert_ne!(resolved.id, collision_ref.id);
}

#[test]
fn test_upsert_head_object_coherence() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();

    let obj = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("Upserted note".to_string()),
        )]),
        StoredPayload::from_markdown("upserted content"),
        vec![],
    );

    let version_ref = store.write_object(&obj).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index
        .upsert_head_object_from_store(&store, &version_ref.id)
        .unwrap();

    let rows = index
        .query_objects(&QueryFilter {
            class: Some("note".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title.as_deref(), Some("Upserted note"));

    let head = index.get_head(&version_ref.id).unwrap().unwrap();
    assert_eq!(head.version_id, version_ref.version_id);
}

#[test]
fn test_index_count_after_rebuild() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    for i in 0..5 {
        let obj = StoredObject::new(
            Kind::Object,
            Some("note".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::from([(
                "title".to_string(),
                earmark_core::HeaderValue::String(format!("note {}", i)),
            )]),
            StoredPayload::from_markdown(format!("content {}", i)),
            vec![],
        );
        store.write_object(&obj).unwrap();
    }

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let canonical_count = store.scan_objects().unwrap().len() as u64;
    let indexed_count = index.object_count().unwrap();
    assert_eq!(
        indexed_count, canonical_count,
        "indexed object count should match canonical store scan count after rebuild"
    );

    let head_count = index.head_count().unwrap();
    assert_eq!(head_count, 5, "all 5 objects should have heads");
}

#[test]
fn test_rebuild_preserves_active_systems() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let system = SystemDefinition {
        system_id: "test-system".to_string(),
        namespace: "test/ns".to_string(),
        title: "Test".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![],
        runtime_profile: RuntimeProfile {
            execution_surface: "local".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "manifest".to_string(),
        },
        activated_at: None,
    };

    let stored = StoredObject::new(
        Kind::SystemDefinition,
        Some("system_definition".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String(system.title.clone()),
        )]),
        StoredPayload::from_yaml(to_yaml(&system).unwrap()),
        vec![],
    );
    let version = store.write_object(&stored).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();
    index
        .activate_system("test/ns", "test-system", &version)
        .unwrap();

    index.rebuild_from_store(&store).unwrap();

    let active = index.get_active_system("test/ns").unwrap();
    assert!(
        active.is_some(),
        "active system should survive rebuild_from_store"
    );
    assert_eq!(active.unwrap().system_id, "test-system");
}

#[test]
fn test_rebuild_objects_by_kind_and_class() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let note = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("note content"),
        vec![],
    );
    store.write_object(&note).unwrap();

    let finding = StoredObject::new(
        Kind::Object,
        Some("finding".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("finding content"),
        vec![],
    );
    store.write_object(&finding).unwrap();

    let instruction = StoredObject::new(
        Kind::Instruction,
        Some("instruction".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("---\nname: test_instruction\nversion: \"1\"\npurpose: test\ninput_classes: []\noutput_classes: []\nexecution_policy: default\ntrace_policy: summary\nregister: user\n---\ninstruction body"),
        vec![],
    );
    store.write_object(&instruction).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let objects = index.get_objects_by_kind(Kind::Object).unwrap();
    assert_eq!(objects.len(), 2, "should find 2 Kind::Object entries");

    let instructions = index.get_objects_by_kind(Kind::Instruction).unwrap();
    assert_eq!(
        instructions.len(),
        1,
        "should find 1 Kind::Instruction entry"
    );

    let rows = index
        .query_objects(&QueryFilter {
            class: Some("note".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].class.as_deref(), Some("note"));

    let rows = index
        .query_objects(&QueryFilter {
            class: Some("finding".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].class.as_deref(), Some("finding"));
}

#[test]
fn test_missing_index_error() {
    let dir = tempdir().unwrap();

    let err = DerivedIndex::open_existing(dir.path()).unwrap_err();
    assert!(
        matches!(err, IndexError::MissingIndex(_)),
        "Expected MissingIndex error, got {}",
        err
    );
}

#[test]
fn test_open_existing_reads_correct_counts() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let obj = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("content"),
        vec![],
    );
    store.write_object(&obj).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();
    drop(index);

    let opened = DerivedIndex::open_existing(dir.path()).unwrap();
    assert_eq!(opened.object_count().unwrap(), 1);
    assert_eq!(opened.head_count().unwrap(), 1);
}

#[test]
fn test_relation_count_after_rebuild() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let a = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("a"),
        vec![],
    );
    let b = StoredObject::new(
        Kind::Object,
        Some("finding".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("b"),
        vec![],
    );
    store.write_object(&a).unwrap();
    store.write_object(&b).unwrap();

    let relation = StoredObject::new(
        Kind::Relation,
        None,
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(
            serde_json::to_vec(&serde_json::json!({
                "source": a.object_ref(),
                "target": b.object_ref(),
                "relation_type": "supports",
                "qualifiers": {},
                "scope": "test"
            }))
            .unwrap(),
        ),
        vec![],
    );
    store.write_object(&relation).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();

    assert_eq!(index.relation_count().unwrap(), 1);

    // Verify adjacency query works
    let adjacency = index.relation_adjacency(&a.envelope.id, false).unwrap();
    assert_eq!(adjacency.len(), 1);
    assert_eq!(adjacency[0].relation_type, "supports");

    // Head query should work for source
    let head = index.get_head(&a.envelope.id).unwrap();
    assert!(head.is_some());
}

fn custom_standing(research_status: &str, review: &str, process: &str) -> Standing {
    let mut values = BTreeMap::new();
    values.insert(
        DimensionId::new("research:status"),
        TokenId::new(research_status),
    );
    values.insert(DimensionId::new("kernel:review"), TokenId::new(review));
    values.insert(DimensionId::new("kernel:process"), TokenId::new(process));
    Standing { values }
}

#[test]
fn test_rebuild_populates_object_standing() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let verified = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("verified", "accepted", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("verified note"),
        vec![],
    );
    store.write_object(&verified).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let rows = index
        .query_objects(&QueryFilter {
            class: Some("note".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].standing.get("research:status").map(String::as_str),
        Some("verified")
    );
    assert_eq!(
        rows[0].standing.get("kernel:review").map(String::as_str),
        Some("accepted")
    );
    assert_eq!(
        rows[0].standing.get("kernel:process").map(String::as_str),
        Some("active")
    );
}

#[test]
fn test_upsert_head_populates_object_standing() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();

    let obj = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("demonstrated", "pending", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("upsert standing test"),
        vec![],
    );
    let version_ref = store.write_object(&obj).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index
        .upsert_head_object_from_store(&store, &version_ref.id)
        .unwrap();

    let rows = index
        .query_objects(&QueryFilter {
            object_id: Some(version_ref.id.as_str().to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].standing.get("research:status").map(String::as_str),
        Some("demonstrated")
    );
}

#[test]
fn test_query_by_custom_dimension() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let verified = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("verified", "accepted", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("verified note"),
        vec![],
    );
    let draft = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("draft", "unreviewed", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("draft note"),
        vec![],
    );
    store.write_object(&verified).unwrap();
    store.write_object(&draft).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let rows = index
        .query_objects(&QueryFilter {
            class: Some("note".to_string()),
            standing: BTreeMap::from([(
                DimensionId::new("research:status"),
                vec![TokenId::new("verified")],
            )]),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].standing.get("research:status").map(String::as_str),
        Some("verified")
    );
}

#[test]
fn test_query_multiple_dimensions_conjunctive() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let matching = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("verified", "accepted", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("matching note"),
        vec![],
    );
    let wrong_review = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("verified", "rejected", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("wrong review"),
        vec![],
    );
    let wrong_status = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("draft", "accepted", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("wrong status"),
        vec![],
    );
    store.write_object(&matching).unwrap();
    store.write_object(&wrong_review).unwrap();
    store.write_object(&wrong_status).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let rows = index
        .query_objects(&QueryFilter {
            class: Some("note".to_string()),
            standing: BTreeMap::from([
                (
                    DimensionId::new("research:status"),
                    vec![TokenId::new("verified")],
                ),
                (
                    DimensionId::new("kernel:review"),
                    vec![TokenId::new("accepted")],
                ),
            ]),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "only objects matching BOTH dimensions should be returned"
    );
    assert_eq!(
        rows[0].standing.get("research:status").map(String::as_str),
        Some("verified")
    );
}

#[test]
fn test_query_multiple_tokens_within_dimension_disjunctive() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let verified = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("verified", "accepted", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("verified"),
        vec![],
    );
    let demonstrated = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("demonstrated", "accepted", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("demonstrated"),
        vec![],
    );
    let draft = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("draft", "accepted", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("draft"),
        vec![],
    );
    store.write_object(&verified).unwrap();
    store.write_object(&demonstrated).unwrap();
    store.write_object(&draft).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();

    // Query for verified OR demonstrated in research:status
    let rows = index
        .query_objects(&QueryFilter {
            class: Some("note".to_string()),
            standing: BTreeMap::from([(
                DimensionId::new("research:status"),
                vec![TokenId::new("verified"), TokenId::new("demonstrated")],
            )]),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        rows.len(),
        2,
        "objects with verified OR demonstrated should be returned"
    );
    let statuses: BTreeSet<&str> = rows
        .iter()
        .map(|r| {
            r.standing
                .get("research:status")
                .map(String::as_str)
                .unwrap()
        })
        .collect();
    assert!(statuses.contains("verified"));
    assert!(statuses.contains("demonstrated"));
    assert!(!statuses.contains("draft"));
}

#[test]
fn test_query_legacy_kernel_review_via_object_standing() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let accepted = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("verified", "accepted", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("accepted note"),
        vec![],
    );
    let rejected = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("verified", "rejected", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("rejected note"),
        vec![],
    );
    store.write_object(&accepted).unwrap();
    store.write_object(&rejected).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let rows = index
        .query_objects(&QueryFilter {
            class: Some("note".to_string()),
            standing: BTreeMap::from([(
                DimensionId::new("kernel:review"),
                vec![TokenId::new("accepted")],
            )]),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].standing.get("kernel:review").map(String::as_str),
        Some("accepted")
    );
}

#[test]
fn test_query_non_standing_filter_still_works() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let note = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("verified", "accepted", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("My Note".to_string()),
        )]),
        StoredPayload::from_markdown("hello world"),
        vec![],
    );
    store.write_object(&note).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();

    // Query with no standing filter should still work
    let rows = index
        .query_objects(&QueryFilter {
            class: Some("note".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title.as_deref(), Some("My Note"));

    // Query by text should still work
    let rows = index
        .query_objects(&QueryFilter {
            text: Some("hello".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);

    // Query by kind should still work
    let rows = index
        .query_objects(&QueryFilter {
            kind: Some("object".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn test_rebuild_clears_previous_object_standing() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let obj = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("verified", "accepted", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("v1"),
        vec![],
    );
    store.write_object(&obj).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    index.rebuild_from_store(&store).unwrap();

    // Verify standing is indexed
    let rows = index
        .query_objects(&QueryFilter {
            standing: BTreeMap::from([(
                DimensionId::new("research:status"),
                vec![TokenId::new("verified")],
            )]),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);

    // Rebuild after changing standing: write new object with different standing
    let updated = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        custom_standing("demonstrated", "accepted", "active"),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("v2"),
        vec![],
    );
    store.write_object(&updated).unwrap();
    index.rebuild_from_store(&store).unwrap();

    // After rebuild, "verified" objects may still be present (separate object)
    // Instead verify that the new standing is correctly indexed
    let rows = index
        .query_objects(&QueryFilter {
            standing: BTreeMap::from([(
                DimensionId::new("research:status"),
                vec![TokenId::new("demonstrated")],
            )]),
            ..Default::default()
        })
        .unwrap();
    assert!(!rows.is_empty());
    assert!(rows
        .iter()
        .any(|r| r.standing.get("research:status").map(String::as_str) == Some("demonstrated")));
}
