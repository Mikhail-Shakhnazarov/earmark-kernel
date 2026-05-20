use earmark_core::{
    ClassDefinition, ClassStandingRules, JsonSchemaRef, Kind, ObjectId, ObjectRef, Provenance,
    RelationPayload, Standing, VersionId, VersionRef, REL_TYPE_USED_INSTRUCTION,
};
use earmark_index::DerivedIndex;
use earmark_store::{GitCanonicalStore, ObjectStore, StoredObject, StoredPayload, WorkspaceLayout};
use std::collections::{BTreeMap, HashMap};
use tempfile::tempdir;

fn setup_store_and_index() -> (tempfile::TempDir, GitCanonicalStore, DerivedIndex) {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();
    (dir, store, index)
}

fn create_object(
    store: &GitCanonicalStore,
    kind: Kind,
    class: Option<&str>,
    body: &str,
) -> ObjectRef {
    let stored = StoredObject::new(
        kind,
        class.map(|s| s.to_string()),
        Standing::default(),
        Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_markdown(body),
        vec![],
    );
    store.write_object(&stored).unwrap();
    stored.object_ref()
}

fn empty_class_definition(name: &str) -> ClassDefinition {
    ClassDefinition {
        name: name.to_string(),
        version: "1".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![],
        validators: vec![],
    }
}

fn register_class(
    store: &GitCanonicalStore,
    index: &DerivedIndex,
    name: &str,
    relation_rules: Vec<earmark_core::RelationRule>,
) {
    let def = ClassDefinition {
        name: name.to_string(),
        version: "1".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules,
        validators: vec![],
    };
    let json = serde_json::to_string(&def).unwrap();
    let stored = StoredObject::new(
        Kind::Object,
        Some("class_definition".to_string()),
        Standing::default(),
        Provenance::direct_input("system"),
        BTreeMap::new(),
        StoredPayload::from_markdown(&json),
        vec![],
    );
    let obj_ref = store.write_object(&stored).unwrap();
    index
        .upsert_head_object_from_store(store, &obj_ref.id)
        .unwrap();
}

#[test]
fn test_endpoint_identity_verification() {
    let (_dir, store, index) = setup_store_and_index();

    let source_ref = create_object(&store, Kind::Object, Some("note"), "source");
    let target_ref = create_object(&store, Kind::Object, Some("finding"), "target");

    // Forge a payload with a different class for target
    let forged_target = ObjectRef::new(
        target_ref.id.clone(),
        target_ref.version_id.clone(),
        Kind::Object,
        Some("forged_class".to_string()),
    );

    let payload = RelationPayload {
        source: source_ref.clone(),
        target: forged_target,
        relation_type: "derived_from".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    let rel_ref = earmark_exec::persist_relation_canonical(
        &store,
        &index,
        payload,
        Provenance::direct_input("runtime"), // use trusted provenance so it doesn't fail auth check
        earmark_core::RelationCreationMode::PrivilegedSystem, // derived_from is now privileged
        None,
    )
    .unwrap();

    let mut failures = Vec::new();
    let mut info = Vec::new();
    let stored_rel = store
        .read_version(&VersionRef::new(
            rel_ref.id.clone(),
            rel_ref.version_id.clone(),
        ))
        .unwrap();

    earmark_exec::validation::validate_relation_object(
        &store,
        &rel_ref.id,
        &stored_rel,
        &HashMap::new(),
        &mut failures,
        &mut info,
    )
    .unwrap();

    assert!(failures.iter().any(|f| f.contains("target class mismatch")));
}

#[test]
fn test_source_authorized_relation() {
    let (_dir, store, index) = setup_store_and_index();

    let source_ref = create_object(&store, Kind::Object, Some("note"), "source");
    let target_ref = create_object(&store, Kind::Object, Some("finding"), "target");

    register_class(
        &store,
        &index,
        "note",
        vec![earmark_core::RelationRule {
            relation_type: "mentions".to_string(),
            counterparty_classes: vec!["finding".to_string()],
            direction: Some("outgoing".to_string()),
            authorizing_endpoint: Some("source".to_string()),
        }],
    );
    register_class(&store, &index, "finding", vec![]);

    let payload = RelationPayload {
        source: source_ref.clone(),
        target: target_ref.clone(),
        relation_type: "mentions".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    let rel_ref = earmark_exec::persist_relation_canonical(
        &store,
        &index,
        payload,
        Provenance::direct_input("test"),
        earmark_core::RelationCreationMode::Declared,
        None,
    )
    .unwrap();

    let mut declared_classes = HashMap::new();
    let mut def = empty_class_definition("note");
    def.relation_rules = vec![earmark_core::RelationRule {
        relation_type: "mentions".to_string(),
        counterparty_classes: vec!["finding".to_string()],
        direction: Some("outgoing".to_string()),
        authorizing_endpoint: Some("source".to_string()),
    }];
    declared_classes.insert("note".to_string(), def);

    let mut failures = Vec::new();
    let mut info = Vec::new();
    let stored_rel = store
        .read_version(&VersionRef::new(
            rel_ref.id.clone(),
            rel_ref.version_id.clone(),
        ))
        .unwrap();

    earmark_exec::validation::validate_relation_object(
        &store,
        &rel_ref.id,
        &stored_rel,
        &declared_classes,
        &mut failures,
        &mut info,
    )
    .unwrap();

    assert!(failures.is_empty(), "Failures: {:?}", failures);
    assert!(info
        .iter()
        .any(|i| i.contains("authorized by source class 'note' outgoing rule")));
}

#[test]
fn test_target_authorized_relation() {
    let (_dir, store, index) = setup_store_and_index();

    let source_ref = create_object(&store, Kind::Object, Some("note"), "source");
    let target_ref = create_object(&store, Kind::Object, Some("finding"), "target");

    register_class(
        &store,
        &index,
        "finding",
        vec![earmark_core::RelationRule {
            relation_type: "references".to_string(),
            counterparty_classes: vec!["note".to_string()],
            direction: Some("incoming".to_string()),
            authorizing_endpoint: Some("target".to_string()),
        }],
    );
    register_class(&store, &index, "note", vec![]);

    let payload = RelationPayload {
        source: source_ref.clone(),
        target: target_ref.clone(),
        relation_type: "references".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    let rel_ref = earmark_exec::persist_relation_canonical(
        &store,
        &index,
        payload,
        Provenance::direct_input("test"),
        earmark_core::RelationCreationMode::Declared,
        None,
    )
    .unwrap();

    let mut declared_classes = HashMap::new();
    let mut def = empty_class_definition("finding");
    def.relation_rules = vec![earmark_core::RelationRule {
        relation_type: "references".to_string(),
        counterparty_classes: vec!["note".to_string()],
        direction: Some("incoming".to_string()),
        authorizing_endpoint: Some("target".to_string()),
    }];
    declared_classes.insert("finding".to_string(), def);

    let mut failures = Vec::new();
    let mut info = Vec::new();
    let stored_rel = store
        .read_version(&VersionRef::new(
            rel_ref.id.clone(),
            rel_ref.version_id.clone(),
        ))
        .unwrap();

    earmark_exec::validation::validate_relation_object(
        &store,
        &rel_ref.id,
        &stored_rel,
        &declared_classes,
        &mut failures,
        &mut info,
    )
    .unwrap();

    assert!(failures.is_empty(), "Failures: {:?}", failures);
    assert!(info
        .iter()
        .any(|i| i.contains("authorized by target class 'finding' incoming rule")));
}

#[test]
fn test_either_endpoint_authorization() {
    let (_dir, store, index) = setup_store_and_index();

    let source_ref = create_object(&store, Kind::Object, Some("note"), "source");
    let target_ref = create_object(&store, Kind::Object, Some("finding"), "target");

    register_class(
        &store,
        &index,
        "finding",
        vec![earmark_core::RelationRule {
            relation_type: "linked_to".to_string(),
            counterparty_classes: vec!["note".to_string()],
            direction: Some("incoming".to_string()),
            authorizing_endpoint: Some("either_endpoint".to_string()),
        }],
    );
    register_class(&store, &index, "note", vec![]);

    let payload = RelationPayload {
        source: source_ref.clone(),
        target: target_ref.clone(),
        relation_type: "linked_to".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    let rel_ref = earmark_exec::persist_relation_canonical(
        &store,
        &index,
        payload,
        Provenance::direct_input("test"),
        earmark_core::RelationCreationMode::Declared,
        None,
    )
    .unwrap();

    let mut declared_classes = HashMap::new();
    let mut def = empty_class_definition("finding");
    def.relation_rules = vec![earmark_core::RelationRule {
        relation_type: "linked_to".to_string(),
        counterparty_classes: vec!["note".to_string()],
        direction: Some("incoming".to_string()),
        authorizing_endpoint: Some("either_endpoint".to_string()),
    }];
    declared_classes.insert("finding".to_string(), def);

    let mut failures = Vec::new();
    let mut info = Vec::new();
    let stored_rel = store
        .read_version(&VersionRef::new(
            rel_ref.id.clone(),
            rel_ref.version_id.clone(),
        ))
        .unwrap();

    earmark_exec::validation::validate_relation_object(
        &store,
        &rel_ref.id,
        &stored_rel,
        &declared_classes,
        &mut failures,
        &mut info,
    )
    .unwrap();

    assert!(failures.is_empty(), "Failures: {:?}", failures);
    assert!(info
        .iter()
        .any(|i| i.contains("authorized by target class 'finding' either_endpoint rule")));
}

#[test]
fn test_privileged_protection() {
    let (_dir, store, index) = setup_store_and_index();

    let source_ref = create_object(&store, Kind::Object, Some("note"), "source");
    let target_ref = create_object(&store, Kind::Instruction, Some("instruction"), "target");

    let payload = RelationPayload {
        source: source_ref.clone(),
        target: target_ref.clone(),
        relation_type: REL_TYPE_USED_INSTRUCTION.to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    // Attempt to create privileged relation as 'Declared' should now fail at persistence time
    let res = earmark_exec::persist_relation_canonical(
        &store,
        &index,
        payload,
        Provenance::direct_input("test"),
        earmark_core::RelationCreationMode::Declared,
        None,
    );

    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .to_string()
        .contains("is a privileged system relation and cannot be created in 'declared' mode"));
}

#[test]
fn test_malformed_rule_fail_fast() {
    let (_dir, store, index) = setup_store_and_index();

    let source_ref = create_object(&store, Kind::Object, Some("note"), "source");
    let target_ref = create_object(&store, Kind::Object, Some("finding"), "target");

    register_class(
        &store,
        &index,
        "note",
        vec![earmark_core::RelationRule {
            relation_type: "mentions".to_string(),
            counterparty_classes: vec!["finding".to_string()],
            direction: Some("outgoing".to_string()),
            authorizing_endpoint: Some("source".to_string()),
        }],
    );
    register_class(&store, &index, "finding", vec![]);

    let payload = RelationPayload {
        source: source_ref.clone(),
        target: target_ref.clone(),
        relation_type: "mentions".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    let rel_ref = earmark_exec::persist_relation_canonical(
        &store,
        &index,
        payload,
        Provenance::direct_input("test"),
        earmark_core::RelationCreationMode::Declared,
        None,
    )
    .unwrap();

    let mut declared_classes = HashMap::new();
    let mut def = empty_class_definition("note");
    def.relation_rules = vec![
        earmark_core::RelationRule {
            relation_type: "mentions".to_string(),
            counterparty_classes: vec!["finding".to_string()],
            direction: Some("INVALID".to_string()),
            authorizing_endpoint: Some("source".to_string()),
        },
        earmark_core::RelationRule {
            relation_type: "mentions".to_string(),
            counterparty_classes: vec!["finding".to_string()],
            direction: Some("outgoing".to_string()),
            authorizing_endpoint: Some("source".to_string()),
        },
    ];
    declared_classes.insert("note".to_string(), def);

    let mut failures = Vec::new();
    let mut info = Vec::new();
    let stored_rel = store
        .read_version(&VersionRef::new(
            rel_ref.id.clone(),
            rel_ref.version_id.clone(),
        ))
        .unwrap();

    earmark_exec::validation::validate_relation_object(
        &store,
        &rel_ref.id,
        &stored_rel,
        &declared_classes,
        &mut failures,
        &mut info,
    )
    .unwrap();

    assert!(failures
        .iter()
        .any(|f| f.contains("malformed matching rule") && f.contains("invalid direction")));
}

#[test]
fn test_block_privileged_type_in_declared_mode() {
    let (_dir, store, index) = setup_store_and_index();

    let source_ref = create_object(&store, Kind::Object, Some("note"), "source");
    let target_ref = create_object(&store, Kind::Instruction, Some("instruction"), "target");

    let payload = RelationPayload {
        source: source_ref.clone(),
        target: target_ref.clone(),
        relation_type: REL_TYPE_USED_INSTRUCTION.to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    // Attempting to persist with Declared mode for a privileged type should fail immediately
    let res = earmark_exec::persist_relation_canonical(
        &store,
        &index,
        payload,
        Provenance::direct_input("test"),
        earmark_core::RelationCreationMode::Declared,
        None,
    );

    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("is a privileged system relation and cannot be created in 'declared' mode")
    );
}

#[test]
fn test_reject_forged_privileged_provenance() {
    let (_dir, store, index) = setup_store_and_index();

    let source_ref = create_object(&store, Kind::Object, Some("note"), "source");
    let target_ref = create_object(&store, Kind::Instruction, Some("instruction"), "target");

    let payload = RelationPayload {
        source: source_ref.clone(),
        target: target_ref.clone(),
        relation_type: REL_TYPE_USED_INSTRUCTION.to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    // Manually create an object with privileged_system mode but untrusted actor ("operator")
    let mut headers = BTreeMap::new();
    headers.insert(
        "relation_creation_mode".to_string(),
        earmark_core::HeaderValue::String("privileged_system".to_string()),
    );

    let stored = StoredObject::new(
        Kind::Relation,
        None,
        Standing::default(),
        Provenance::direct_input("operator"), // untrusted actor
        headers,
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&payload).unwrap()),
        vec![],
    );

    let rel_ref = store.write_object(&stored).unwrap();
    index
        .upsert_head_object_from_store(&store, &rel_ref.id)
        .unwrap();

    let mut failures = Vec::new();
    let mut info = Vec::new();
    let stored_rel = store
        .read_version(&VersionRef::new(
            rel_ref.id.clone(),
            rel_ref.version_id.clone(),
        ))
        .unwrap();

    earmark_exec::validation::validate_relation_object(
        &store,
        &rel_ref.id,
        &stored_rel,
        &HashMap::new(),
        &mut failures,
        &mut info,
    )
    .unwrap();

    assert!(failures
        .iter()
        .any(|f| f.contains("has untrusted provenance")));
}

#[test]
fn test_missing_endpoint_version_failure() {
    let (_dir, store, index) = setup_store_and_index();

    let source_ref = create_object(&store, Kind::Object, Some("note"), "source");

    // Target version does not exist in store
    let missing_target = ObjectRef::new(
        ObjectId::new(),
        VersionId::new(),
        Kind::Object,
        Some("finding".to_string()),
    );

    let payload = RelationPayload {
        source: source_ref.clone(),
        target: missing_target,
        relation_type: "derived_from".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    // We cannot persist it: authorization/endpoint checking fails fast at write time
    let res = earmark_exec::persist_relation_canonical(
        &store,
        &index,
        payload,
        Provenance::direct_input("runtime"),
        earmark_core::RelationCreationMode::PrivilegedSystem,
        None,
    );

    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(err.contains("failed to load target endpoint"));
}
