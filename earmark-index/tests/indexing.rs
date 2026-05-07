use std::collections::BTreeMap;

use earmark_core::{to_yaml, Kind, Provenance, RuntimeProfile, Standing, SystemDefinition};
use earmark_index::{DerivedIndex, QueryFilter};
use earmark_store::{CanonicalStore, GitCanonicalStore, StoredObject, StoredPayload};
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
    let adjacency = index.relation_adjacency(&a.envelope.id).unwrap();
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
