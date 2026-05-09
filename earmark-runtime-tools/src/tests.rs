use super::*;
use chrono::Utc;
use earmark_connected_context::{CompiledContextCompiler, WorkSurfaceManifest};
use earmark_core::{
    ClassFilter, Kind, Provenance, RelationFilter, Standing, StandingFilter, TransitionAssignmentId,
};
use earmark_exec::ProviderRegistry;
use earmark_index::DerivedIndex;
use earmark_store::{CanonicalStore, GitCanonicalStore, StoredObject, StoredPayload};
use serde_json::json;
use std::collections::BTreeMap;
use tempfile::tempdir;

fn setup_surface(dir: &std::path::Path) -> (GitCanonicalStore, DerivedIndex, ProviderRegistry) {
    let store = GitCanonicalStore::new(dir);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir).unwrap();
    let registry = ProviderRegistry::default();
    (store, index, registry)
}

fn register_class_definition(
    store: &GitCanonicalStore,
    index: &DerivedIndex,
    name: &str,
    rules: Vec<earmark_core::RelationRule>,
) -> earmark_core::VersionRef {
    let class_def = earmark_core::ClassDefinition {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: earmark_core::JsonSchemaRef("inline:any".to_string()),
        standing_rules: earmark_core::ClassStandingRules::default(),
        relation_rules: rules,
        validators: vec![],
    };

    let payload =
        earmark_store::StoredPayload::from_yaml(earmark_core::to_yaml(&class_def).unwrap());
    let stored = earmark_store::StoredObject::new(
        earmark_core::Kind::Object,
        Some("class_definition".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance {
            actor: "system".to_string(),
            source_type: "manual".to_string(),
            source_ref: None,
            lineage: vec![],
            import_path: None,
            captured_at: Utc::now(),
        },
        BTreeMap::new(),
        payload,
        vec![],
    );

    let version_ref = store.write_object(&stored).unwrap();
    index
        .upsert_head_object_from_store(store, &version_ref.id)
        .unwrap();
    version_ref
}

fn register_simple_class(
    store: &GitCanonicalStore,
    index: &DerivedIndex,
    name: &str,
    rel_type: &str,
    target_class: &str,
) {
    register_class_definition(
        store,
        index,
        name,
        vec![earmark_core::RelationRule {
            relation_type: rel_type.to_string(),
            counterparty_classes: vec![target_class.to_string()],
            direction: None,
            authorizing_endpoint: None,
        }],
    );
}

struct FakeContextCompiler;

impl<S: CanonicalStore> CompiledContextCompiler<S> for FakeContextCompiler {
    fn compile(
        &self,
        _store: &S,
        _index: &DerivedIndex,
        template_ref: &earmark_core::VersionRef,
        _work_packet: Option<earmark_core::ObjectRef>,
    ) -> Result<WorkSurfaceManifest, earmark_connected_context::ProjectError> {
        Ok(WorkSurfaceManifest {
            surface_id: "fake_surface".to_string(),
            compiled_context: template_ref.clone(),
            work_packet: None,
            generated_at: Utc::now(),
            objects: vec![],
            boundary_relations: vec![],
            constraints: BTreeMap::new(),
        })
    }
}

#[test]
fn test_compile_work_surface_supports_context_compiler_substitution() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };
    let compiler = FakeContextCompiler;
    let template_ref = earmark_core::VersionRef::new(
        earmark_core::ObjectId::parse("obj_00000000000000000000000000000001").unwrap(),
        earmark_core::VersionId::parse("ver_00000000000000000000000000000001").unwrap(),
    );
    let manifest = surface
        .compile_work_surface_with(&compiler, &template_ref)
        .unwrap();
    assert_eq!(manifest.surface_id, "fake_surface");
    assert_eq!(manifest.compiled_context.id, template_ref.id);
}

#[test]
fn test_duplicate_active_assignment_rejection() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let _assignment1 = surface
        .assign_transition(
            "run1".to_string(),
            "trans1".to_string(),
            "agentA".to_string(),
            vec![],
            None,
        )
        .unwrap();

    let err = surface
        .assign_transition(
            "run1".to_string(),
            "trans1".to_string(),
            "agentB".to_string(),
            vec![],
            None,
        )
        .unwrap_err();
    assert!(matches!(err, RuntimeToolError::Conflict(_)));
}

#[test]
fn test_assignment_completion_creating_a_change_set() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let assignment = surface
        .assign_transition(
            "run1".to_string(),
            "trans1".to_string(),
            "agentA".to_string(),
            vec![],
            None,
        )
        .unwrap();

    let draft = ChangeSetDraft {
        created_objects: vec![],
        created_relations: vec![],
        updated_objects: vec![],
        governance_events: vec![],
        standing_requests: vec![],
        blocked_operations: vec![],
        unresolved_ambiguities: vec![],
        rejected_candidates: vec![],
    };

    let change_set = surface
        .complete_transition_assignment(assignment.id.clone(), draft, "agentA".to_string())
        .unwrap();
    assert_eq!(change_set.assignment_id, Some(assignment.id));
    assert_eq!(change_set.run_id, "run1");
}

#[test]
fn test_loading_missing_handoff_manifest() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let err = surface
        .load_handoff(
            earmark_core::HandoffManifestId::parse("obj_00000000000000000000000000000001").unwrap(),
        )
        .unwrap_err();
    assert!(matches!(err, RuntimeToolError::MissingObject(_)));
}

#[test]
fn test_relation_qualifier_json_conversion_failure() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    register_class_definition(
        &store,
        &index,
        "test",
        vec![earmark_core::RelationRule {
            relation_type: "rel".to_string(),
            counterparty_classes: vec!["test".to_string()],
            direction: None,
            authorizing_endpoint: None,
        }],
    );

    let obj1 = surface
        .deposit_object(
            "test".to_string(),
            None,
            None,
            json!("body"),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
            DepositValidationContext::default(),
        )
        .unwrap();
    let obj2 = surface
        .deposit_object(
            "test".to_string(),
            None,
            None,
            json!("body"),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
            DepositValidationContext::default(),
        )
        .unwrap();

    let metadata = json!({
        "bad_qualifier": ["nested", "array"]
    });

    let err = surface
        .create_relation(
            obj1.id.clone(),
            obj2.id.clone(),
            "rel".to_string(),
            metadata,
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
        )
        .unwrap_err();
    assert!(matches!(err, RuntimeToolError::Json(_)));
}

#[test]
fn test_compile_connected_context_honors_depth_and_filters() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    register_simple_class(&store, &index, "root", "supports", "mid");
    register_simple_class(&store, &index, "mid", "blocks", "far");
    register_class_definition(&store, &index, "far", vec![]);

    let root = surface
        .deposit_object(
            "root".to_string(),
            None,
            Some("Root".to_string()),
            json!("root body"),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
            DepositValidationContext::default(),
        )
        .unwrap();
    let mid = surface
        .deposit_object(
            "mid".to_string(),
            None,
            Some("Mid".to_string()),
            json!("mid body"),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
            DepositValidationContext::default(),
        )
        .unwrap();
    let far = surface
        .deposit_object(
            "far".to_string(),
            None,
            Some("Far".to_string()),
            json!("far body"),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
            DepositValidationContext::default(),
        )
        .unwrap();

    let rel1 = surface
        .create_relation(
            root.id.clone(),
            mid.id.clone(),
            "supports".to_string(),
            json!({}),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
        )
        .unwrap();
    surface
        .create_relation(
            mid.id.clone(),
            far.id.clone(),
            "blocks".to_string(),
            json!({}),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
        )
        .unwrap();

    let manifest = surface
        .compile_connected_context(
            vec![root.id.clone()],
            1,
            Some(RelationFilter {
                allowed_types: vec!["supports".to_string()],
            }),
            Some(ClassFilter {
                allowed_classes: vec!["mid".to_string()],
            }),
            None,
        )
        .unwrap();

    assert_eq!(manifest.root_object_ids, vec![root.id.clone()]);
    assert_eq!(manifest.object_refs.len(), 2);
    assert!(manifest
        .object_refs
        .iter()
        .any(|object| object.id == root.id));
    assert!(manifest
        .object_refs
        .iter()
        .any(|object| object.id == mid.id));
    assert!(!manifest
        .object_refs
        .iter()
        .any(|object| object.id == far.id));
    assert_eq!(manifest.relation_refs.len(), 1);
    assert_eq!(manifest.relation_refs[0].id, rel1.id);
}

#[test]
fn test_compile_connected_context_respects_standing_filters() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    register_simple_class(&store, &index, "root", "supports", "neighbor");
    register_class_definition(&store, &index, "neighbor", vec![]);

    let root = surface
        .deposit_object(
            "root".to_string(),
            None,
            None,
            json!("root"),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
            DepositValidationContext::default(),
        )
        .unwrap();

    let supported = StoredObject::new(
        Kind::Object,
        Some("neighbor".to_string()),
        Standing {
            epistemic: earmark_core::EpistemicStanding::Supported,
            ..Standing::default()
        },
        Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_markdown("supported"),
        vec![],
    );
    let supported_ref = store.write_object(&supported).unwrap();

    let contested = StoredObject::new(
        Kind::Object,
        Some("neighbor".to_string()),
        Standing {
            epistemic: earmark_core::EpistemicStanding::Contested,
            ..Standing::default()
        },
        Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_markdown("contested"),
        vec![],
    );
    let contested_ref = store.write_object(&contested).unwrap();

    surface
        .create_relation(
            root.id.clone(),
            supported_ref.id.clone(),
            "supports".to_string(),
            json!({}),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
        )
        .unwrap();
    surface
        .create_relation(
            root.id.clone(),
            contested_ref.id.clone(),
            "supports".to_string(),
            json!({}),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
        )
        .unwrap();

    let manifest = surface
        .compile_connected_context(
            vec![root.id.clone()],
            1,
            Some(RelationFilter {
                allowed_types: vec!["supports".to_string()],
            }),
            None,
            Some(StandingFilter {
                allowed_epistemic: vec![earmark_core::EpistemicStanding::Supported],
            }),
        )
        .unwrap();

    assert!(manifest
        .object_refs
        .iter()
        .any(|object| object.id == root.id));
    assert!(manifest
        .object_refs
        .iter()
        .any(|object| object.id == supported_ref.id));
    assert!(!manifest
        .object_refs
        .iter()
        .any(|object| object.id == contested_ref.id));
}

#[test]
fn test_assignment_lifecycle_release() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let assignment = surface
        .assign_transition(
            "run1".to_string(),
            "trans1".to_string(),
            "agentA".to_string(),
            vec![],
            None,
        )
        .unwrap();
    surface.release_assignment(assignment.id.clone()).unwrap();

    let (_, updated) = surface.find_head_assignment(&assignment.id).unwrap();
    assert_eq!(updated.status, earmark_core::AssignmentStatus::Released);
}

#[test]
fn test_assignment_lifecycle_expire() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let assignment = surface
        .assign_transition(
            "run1".to_string(),
            "trans1".to_string(),
            "agentA".to_string(),
            vec![],
            None,
        )
        .unwrap();
    surface.expire_assignment(assignment.id.clone()).unwrap();

    let (_, updated) = surface.find_head_assignment(&assignment.id).unwrap();
    assert_eq!(updated.status, earmark_core::AssignmentStatus::Expired);
}

#[test]
fn test_assignment_lifecycle_supersede() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let assignment = surface
        .assign_transition(
            "run1".to_string(),
            "trans1".to_string(),
            "agentA".to_string(),
            vec![],
            None,
        )
        .unwrap();
    let successor_id =
        TransitionAssignmentId::parse("obj_00000000000000000000000000000002").unwrap();
    surface
        .supersede_assignment(assignment.id.clone(), successor_id)
        .unwrap();

    let (_, updated) = surface.find_head_assignment(&assignment.id).unwrap();
    assert_eq!(updated.status, earmark_core::AssignmentStatus::Superseded);
}

#[test]
fn test_assignment_lifecycle_resume() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let assignment1 = surface
        .assign_transition(
            "run1".to_string(),
            "trans1".to_string(),
            "agentA".to_string(),
            vec![],
            None,
        )
        .unwrap();
    surface.expire_assignment(assignment1.id.clone()).unwrap();

    let resumed = surface
        .resume_assignment(assignment1.id.clone(), "agentB".to_string(), None)
        .unwrap();
    assert_eq!(resumed.status, earmark_core::AssignmentStatus::Assigned);
    assert_eq!(resumed.assigned_to, "agentB");
}

#[test]
fn test_resume_fails_if_active_duplicate_exists() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    // Create expired assignment
    let assignment1 = surface
        .assign_transition(
            "run1".to_string(),
            "trans1".to_string(),
            "agentA".to_string(),
            vec![],
            None,
        )
        .unwrap();
    surface.expire_assignment(assignment1.id.clone()).unwrap();

    // Create parallel active assignment (same run/trans) - this shouldn't be blocked by expired
    let _assignment2 = surface
        .assign_transition(
            "run1".to_string(),
            "trans1".to_string(),
            "agentB".to_string(),
            vec![],
            None,
        )
        .unwrap();

    // Now try to resume assignment1 - should fail because assignment2 is active
    let err = surface
        .resume_assignment(assignment1.id.clone(), "agentC".to_string(), None)
        .unwrap_err();
    assert!(matches!(err, RuntimeToolError::Conflict(_)));
}

#[test]
fn test_compile_connected_context_terminates_on_cycle() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    register_simple_class(&store, &index, "node", "linked", "node");

    let a = surface
        .deposit_object(
            "node".to_string(),
            None,
            None,
            json!("a"),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
            DepositValidationContext::default(),
        )
        .unwrap();
    let b = surface
        .deposit_object(
            "node".to_string(),
            None,
            None,
            json!("b"),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
            DepositValidationContext::default(),
        )
        .unwrap();
    surface
        .create_relation(
            a.id.clone(),
            b.id.clone(),
            "linked".to_string(),
            json!({}),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
        )
        .unwrap();
    surface
        .create_relation(
            b.id.clone(),
            a.id.clone(),
            "linked".to_string(),
            json!({}),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
        )
        .unwrap();

    let manifest = surface
        .compile_connected_context(vec![a.id.clone()], 8, None, None, None)
        .unwrap();
    assert_eq!(manifest.object_refs.len(), 2);
    assert_eq!(manifest.relation_refs.len(), 2);
}

#[test]
fn test_compile_connected_context_dedupes_relation_refs() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    register_simple_class(&store, &index, "node", "linked", "node");

    let a = surface
        .deposit_object(
            "node".to_string(),
            None,
            None,
            json!("a"),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
            DepositValidationContext::default(),
        )
        .unwrap();
    let b = surface
        .deposit_object(
            "node".to_string(),
            None,
            None,
            json!("b"),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
            DepositValidationContext::default(),
        )
        .unwrap();
    let c = surface
        .deposit_object(
            "node".to_string(),
            None,
            None,
            json!("c"),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
            DepositValidationContext::default(),
        )
        .unwrap();

    surface
        .create_relation(
            a.id.clone(),
            b.id.clone(),
            "linked".to_string(),
            json!({}),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
        )
        .unwrap();
    surface
        .create_relation(
            a.id.clone(),
            c.id.clone(),
            "linked".to_string(),
            json!({}),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
        )
        .unwrap();
    surface
        .create_relation(
            b.id.clone(),
            c.id.clone(),
            "linked".to_string(),
            json!({}),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
        )
        .unwrap();

    let manifest = surface
        .compile_connected_context(vec![a.id.clone()], 8, None, None, None)
        .unwrap();
    let unique_relation_ids = manifest
        .relation_refs
        .iter()
        .map(|r| r.id.as_str().to_string())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique_relation_ids.len(), manifest.relation_refs.len());
}

fn create_test_object(
    store: &GitCanonicalStore,
    index: &DerivedIndex,
    class: &str,
) -> earmark_core::ObjectId {
    let stored = earmark_store::StoredObject::new(
        earmark_core::Kind::Object,
        Some(class.to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance {
            actor: "system".to_string(),
            source_type: "manual".to_string(),
            source_ref: None,
            lineage: vec![],
            import_path: None,
            captured_at: Utc::now(),
        },
        BTreeMap::new(),
        earmark_store::StoredPayload::from_json_bytes(vec![b'{', b'}']),
        vec![],
    );
    let version_ref = store.write_object(&stored).unwrap();
    index
        .upsert_head_object_from_store(store, &version_ref.id)
        .unwrap();
    version_ref.id
}

#[test]
fn test_create_relation_enforces_rules() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    // 1. Setup classes
    register_class_definition(
        &store,
        &index,
        "finding",
        vec![earmark_core::RelationRule {
            relation_type: "derived_from".to_string(),
            counterparty_classes: vec!["source_note".to_string()],
            direction: Some("outgoing".to_string()),
            authorizing_endpoint: None,
        }],
    );
    register_class_definition(&store, &index, "source_note", vec![]);
    register_class_definition(&store, &index, "summary", vec![]);

    // 2. Setup objects
    let source_id = create_test_object(&store, &index, "finding");
    let target_note_id = create_test_object(&store, &index, "source_note");
    let target_summary_id = create_test_object(&store, &index, "summary");

    let provenance = earmark_core::RuntimeProvenance {
        actor: "test_actor".to_string(),
        source_type: "test".to_string(),
    };

    // Case 1: Valid relation succeeds
    surface
        .create_relation(
            source_id.clone(),
            target_note_id.clone(),
            "derived_from".to_string(),
            json!({}),
            provenance.clone(),
        )
        .expect("valid relation should succeed");

    // Case 2: Undeclared relation type fails
    let err = surface
        .create_relation(
            source_id.clone(),
            target_note_id,
            "mentions".to_string(),
            json!({}),
            provenance.clone(),
        )
        .unwrap_err();
    assert!(matches!(err, RuntimeToolError::RelationRuleViolation(_)));

    // Case 3: Wrong target class fails
    let err = surface
        .create_relation(
            source_id.clone(),
            target_summary_id,
            "derived_from".to_string(),
            json!({}),
            provenance.clone(),
        )
        .unwrap_err();
    assert!(matches!(err, RuntimeToolError::RelationRuleViolation(_)));
}

#[test]
fn test_create_relation_direction_enforcement() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    register_class_definition(
        &store,
        &index,
        "a",
        vec![
            earmark_core::RelationRule {
                relation_type: "points_to".to_string(),
                counterparty_classes: vec!["b".to_string()],
                direction: Some("incoming".to_string()), // A cannot point to B with this rule
                authorizing_endpoint: None,
            },
            earmark_core::RelationRule {
                relation_type: "both".to_string(),
                counterparty_classes: vec!["b".to_string()],
                direction: Some("bidirectional".to_string()),
                authorizing_endpoint: None,
            },
        ],
    );
    register_class_definition(&store, &index, "b", vec![]);

    let source_id = create_test_object(&store, &index, "a");
    let target_id = create_test_object(&store, &index, "b");

    let provenance = earmark_core::RuntimeProvenance {
        actor: "test_actor".to_string(),
        source_type: "test".to_string(),
    };

    // Incoming rule fails for outgoing creation
    let count_before = surface.index.relation_count().unwrap();
    let err = surface
        .create_relation(
            source_id.clone(),
            target_id.clone(),
            "points_to".to_string(),
            json!({}),
            provenance.clone(),
        )
        .unwrap_err();
    assert!(matches!(err, RuntimeToolError::RelationRuleViolation(_)));
    assert_eq!(surface.index.relation_count().unwrap(), count_before);

    // Bidirectional rule succeeds
    surface
        .create_relation(
            source_id.clone(),
            target_id.clone(),
            "both".to_string(),
            json!({}),
            provenance.clone(),
        )
        .expect("bidirectional should succeed");
    assert_eq!(surface.index.relation_count().unwrap(), count_before + 1);

    // Test unknown direction (bypassing normal declaration validation by writing directly to store)
    let bad_class_def = earmark_core::ClassDefinition {
        name: "bad_direction".to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: earmark_core::JsonSchemaRef("inline:any".to_string()),
        standing_rules: earmark_core::ClassStandingRules::default(),
        relation_rules: vec![earmark_core::RelationRule {
            relation_type: "any".to_string(),
            counterparty_classes: vec!["b".to_string()],
            direction: Some("invalid".to_string()),
            authorizing_endpoint: None,
        }],
        validators: vec![],
    };

    let payload =
        earmark_store::StoredPayload::from_yaml(earmark_core::to_yaml(&bad_class_def).unwrap());
    let stored = earmark_store::StoredObject::new(
        earmark_core::Kind::Object,
        Some("class_definition".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        payload,
        vec![],
    );
    let class_ref = store.write_object(&stored).unwrap();
    index
        .upsert_head_object_from_store(&store, &class_ref.id)
        .unwrap();

    let source_bad_id = create_test_object(&store, &index, "bad_direction");
    let count_before = surface.index.relation_count().unwrap();
    let err = surface
        .create_relation(
            source_bad_id,
            target_id,
            "any".to_string(),
            json!({}),
            provenance,
        )
        .unwrap_err();

    assert!(matches!(
        err,
        RuntimeToolError::RelationRuleViolation(ref msg) if msg.contains("unknown relation direction: invalid")
    ));
    assert_eq!(surface.index.relation_count().unwrap(), count_before);
}

#[test]
fn test_create_relation_missing_classes_fails() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let provenance = earmark_core::RuntimeProvenance {
        actor: "test_actor".to_string(),
        source_type: "test".to_string(),
    };

    // Case 1: Source has no class
    let source_id = {
        let stored = earmark_store::StoredObject::new(
            earmark_core::Kind::Object,
            None,
            earmark_core::Standing::default(),
            earmark_core::Provenance::direct_input("test"),
            BTreeMap::new(),
            earmark_store::StoredPayload::from_json_bytes(vec![b'{', b'}']),
            vec![],
        );
        let v = store.write_object(&stored).unwrap();
        index.upsert_head_object_from_store(&store, &v.id).unwrap();
        v.id
    };
    let target_id = create_test_object(&store, &index, "some_class");

    let count_before = surface.index.relation_count().unwrap();
    let err = surface
        .create_relation(
            source_id,
            target_id.clone(),
            "any".to_string(),
            json!({}),
            provenance.clone(),
        )
        .unwrap_err();
    assert!(matches!(
        err,
        RuntimeToolError::RelationRuleViolation(ref msg) if msg.contains("no class")
    ));
    assert_eq!(surface.index.relation_count().unwrap(), count_before);

    // Case 2: Missing class definition
    let source_id_with_class = create_test_object(&store, &index, "missing_class");
    let err = surface
        .create_relation(
            source_id_with_class,
            target_id,
            "any".to_string(),
            json!({}),
            provenance,
        )
        .unwrap_err();
    assert!(matches!(err, RuntimeToolError::MissingClassDefinition(_)));
    assert_eq!(surface.index.relation_count().unwrap(), count_before);
}

fn register_system_definition(
    store: &GitCanonicalStore,
    index: &DerivedIndex,
    system_id: &str,
    namespace: &str,
    classes: Vec<VersionRef>,
) -> VersionRef {
    let system_def = earmark_core::SystemDefinition {
        system_id: system_id.to_string(),
        title: system_id.to_string(),
        description: None,
        namespace: namespace.to_string(),
        classes,
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        runtime_profile: earmark_core::RuntimeProfile {
            execution_surface: "local".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "strict".to_string(),
        },
        activated_at: None,
    };

    let payload =
        earmark_store::StoredPayload::from_yaml(earmark_core::to_yaml(&system_def).unwrap());
    let stored = earmark_store::StoredObject::new(
        earmark_core::Kind::SystemDefinition,
        None,
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("system"),
        BTreeMap::new(),
        payload,
        vec![],
    );

    let version_ref = store.write_object(&stored).unwrap();
    index.rebuild_from_store(store).unwrap();
    version_ref
}

#[test]
fn test_deposit_admission_enforcement() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    // 1. Register classes
    let class_a_ref = register_class_definition(&store, &index, "class_a", vec![]);
    let _class_b_ref = register_class_definition(&store, &index, "class_b", vec![]);

    // 2. Register system that only admits class_a
    let system_ref = register_system_definition(
        &store,
        &index,
        "governed_system",
        "governed_ns",
        vec![class_a_ref],
    );

    // 3. Activate system
    index
        .activate_system("governed_ns", "governed_system", &system_ref)
        .unwrap();

    let provenance = RuntimeProvenance {
        actor: "test".to_string(),
        source_type: "test".to_string(),
    };

    // Case 1: Deposit admitted class succeeds
    surface
        .deposit_object(
            "class_a".to_string(),
            None,
            None,
            json!({"foo": "bar"}),
            provenance.clone(),
            DepositValidationContext {
                namespace: Some("governed_ns".to_string()),
            },
        )
        .expect("admitted class should be accepted");

    // Case 2: Deposit non-admitted class fails
    let err = surface
        .deposit_object(
            "class_b".to_string(),
            None,
            None,
            json!({"foo": "bar"}),
            provenance.clone(),
            DepositValidationContext {
                namespace: Some("governed_ns".to_string()),
            },
        )
        .unwrap_err();

    assert!(matches!(
        err,
        RuntimeToolError::AdmissionError {
            ref requested_class,
            ref namespace,
            ref system_id,
        } if requested_class == "class_b" && namespace == "governed_ns" && system_id == "governed_system"
    ));

    // Case 3: Deposit to different (scratch) namespace succeeds even if class not admitted in governed_ns
    surface
        .deposit_object(
            "class_b".to_string(),
            None,
            None,
            json!({"foo": "bar"}),
            provenance.clone(),
            DepositValidationContext {
                namespace: Some("scratch_ns".to_string()),
            },
        )
        .expect("deposit to scratch namespace should be permissive");

    // Case 4: Deposit without namespace succeeds (scratch behavior)
    surface
        .deposit_object(
            "class_b".to_string(),
            None,
            None,
            json!({"foo": "bar"}),
            provenance,
            DepositValidationContext::default(),
        )
        .expect("deposit without namespace should be permissive");
}

#[test]
fn test_deposit_system_integrity_on_broken_class_ref() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    // 1. Create a class ref that doesn't exist in store
    let broken_ref = VersionRef::new(
        earmark_core::ObjectId::parse("obj_00000000000000000000000000000001").unwrap(),
        earmark_core::VersionId::parse("ver_00000000000000000000000000000001").unwrap(),
    );

    // 2. Register system with that broken ref
    let system_ref = register_system_definition(
        &store,
        &index,
        "broken_system",
        "broken_ns",
        vec![broken_ref],
    );

    // 3. Activate
    index
        .activate_system("broken_ns", "broken_system", &system_ref)
        .unwrap();

    // 4. Deposit should fail with SystemIntegrity error
    let err = surface
        .deposit_object(
            "any".to_string(),
            None,
            None,
            json!({}),
            RuntimeProvenance {
                actor: "test".to_string(),
                source_type: "test".to_string(),
            },
            DepositValidationContext {
                namespace: Some("broken_ns".to_string()),
            },
        )
        .unwrap_err();

    assert!(matches!(err, RuntimeToolError::SystemIntegrity(_)));
}

#[test]
fn test_endpoint_authorized_relations() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let provenance = RuntimeProvenance {
        actor: "test".to_string(),
        source_type: "test".to_string(),
    };

    // 1. Source-only authorization (traditional)
    // class_a allows outgoing 'linked_to' to class_b
    register_class_definition(
        &store,
        &index,
        "class_a",
        vec![earmark_core::RelationRule {
            relation_type: "linked_to".to_string(),
            counterparty_classes: vec!["class_b".to_string()],
            direction: Some("outgoing".to_string()),
            authorizing_endpoint: Some("source".to_string()),
        }],
    );
    // class_b allows nothing
    register_class_definition(&store, &index, "class_b", vec![]);

    let obj_a = surface
        .deposit_object(
            "class_a".to_string(),
            None,
            None,
            json!({}),
            provenance.clone(),
            DepositValidationContext::default(),
        )
        .unwrap();
    let obj_b = surface
        .deposit_object(
            "class_b".to_string(),
            None,
            None,
            json!({}),
            provenance.clone(),
            DepositValidationContext::default(),
        )
        .unwrap();

    // Should succeed (source authorized)
    surface
        .create_relation(obj_a.id.clone(), obj_b.id.clone(), "linked_to".to_string(), json!({}), provenance.clone())
        .expect("source authorized relation should succeed");

    // 2. Target-only authorization
    // class_c allows nothing
    register_class_definition(&store, &index, "class_c", vec![]);
    // class_d allows incoming 'mentions' from class_c
    register_class_definition(
        &store,
        &index,
        "class_d",
        vec![earmark_core::RelationRule {
            relation_type: "mentions".to_string(),
            counterparty_classes: vec!["class_c".to_string()],
            direction: Some("incoming".to_string()),
            authorizing_endpoint: Some("target".to_string()),
        }],
    );

    let obj_c = surface
        .deposit_object(
            "class_c".to_string(),
            None,
            None,
            json!({}),
            provenance.clone(),
            DepositValidationContext::default(),
        )
        .unwrap();
    let obj_d = surface
        .deposit_object(
            "class_d".to_string(),
            None,
            None,
            json!({}),
            provenance.clone(),
            DepositValidationContext::default(),
        )
        .unwrap();

    // Should succeed (target authorized)
    let rel_mentions = surface
        .create_relation(obj_c.id.clone(), obj_d.id.clone(), "mentions".to_string(), json!({}), provenance.clone())
        .expect("target authorized relation should succeed");
    
    // Verify headers
    let rel_obj = store.read_version(&rel_mentions.version_ref()).unwrap();
    assert_eq!(rel_obj.envelope.headers.get("relation_auth_endpoint").unwrap().as_string().unwrap(), "target");
    assert_eq!(rel_obj.envelope.headers.get("relation_auth_class").unwrap().as_string().unwrap(), "class_d");
    assert_eq!(rel_obj.envelope.headers.get("relation_auth_authority").unwrap().as_string().unwrap(), "target");

    // 3. Either-endpoint authorization
    // class_e allows bidirectional 'partner' with class_f, either can authorize
    register_class_definition(
        &store,
        &index,
        "class_e",
        vec![earmark_core::RelationRule {
            relation_type: "partner".to_string(),
            counterparty_classes: vec!["class_f".to_string()],
            direction: Some("bidirectional".to_string()),
            authorizing_endpoint: Some("either_endpoint".to_string()),
        }],
    );
    // class_f allows nothing
    register_class_definition(&store, &index, "class_f", vec![]);

    let obj_e = surface
        .deposit_object(
            "class_e".to_string(),
            None,
            None,
            json!({}),
            provenance.clone(),
            DepositValidationContext::default(),
        )
        .unwrap();
    let obj_f = surface
        .deposit_object(
            "class_f".to_string(),
            None,
            None,
            json!({}),
            provenance.clone(),
            DepositValidationContext::default(),
        )
        .unwrap();

    // Should succeed (source authorized via either_endpoint)
    surface
        .create_relation(obj_e.id.clone(), obj_f.id.clone(), "partner".to_string(), json!({}), provenance.clone())
        .expect("either_endpoint authorized relation (source side) should succeed");

    // Should also succeed if we reverse them (target authorized via either_endpoint)
    surface
        .create_relation(obj_f.id.clone(), obj_e.id.clone(), "partner".to_string(), json!({}), provenance.clone())
        .expect("either_endpoint authorized relation (target side) should succeed");

    // 4. Rejection Case
    // class_g allows nothing
    register_class_definition(&store, &index, "class_g", vec![]);
    // class_h allows nothing
    register_class_definition(&store, &index, "class_h", vec![]);

    let obj_g = surface
        .deposit_object(
            "class_g".to_string(),
            None,
            None,
            json!({}),
            provenance.clone(),
            DepositValidationContext::default(),
        )
        .unwrap();
    let obj_h = surface
        .deposit_object(
            "class_h".to_string(),
            None,
            None,
            json!({}),
            provenance.clone(),
            DepositValidationContext::default(),
        )
        .unwrap();

    surface
        .create_relation(obj_g.id.clone(), obj_h.id.clone(), "unauthorized".to_string(), json!({}), provenance)
        .unwrap_err();
}

#[test]
fn test_relation_rule_ordering_is_non_semantic() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let provenance = RuntimeProvenance {
        actor: "test".to_string(),
        source_type: "test".to_string(),
    };

    // class_a has two rules for 'linked_to':
    // 1. rule that DOES NOT match counterparty class_b
    // 2. rule that DOES match counterparty class_b
    register_class_definition(
        &store,
        &index,
        "class_a",
        vec![
            earmark_core::RelationRule {
                relation_type: "linked_to".to_string(),
                counterparty_classes: vec!["other_class".to_string()],
                direction: Some("outgoing".to_string()),
                authorizing_endpoint: Some("source".to_string()),
            },
            earmark_core::RelationRule {
                relation_type: "linked_to".to_string(),
                counterparty_classes: vec!["class_b".to_string()],
                direction: Some("outgoing".to_string()),
                authorizing_endpoint: Some("source".to_string()),
            },
        ],
    );
    register_class_definition(&store, &index, "class_b", vec![]);

    let obj_a = create_test_object(&store, &index, "class_a");
    let obj_b = create_test_object(&store, &index, "class_b");

    // Should succeed because the second rule matches
    surface
        .create_relation(obj_a, obj_b, "linked_to".to_string(), json!({}), provenance)
        .expect("relation should succeed even if earlier rule does not match");
}

#[test]
fn test_malformed_relation_rule_fails_hard() {
    let dir = tempdir().unwrap();
    let (store, index, registry) = setup_surface(dir.path());
    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let provenance = RuntimeProvenance {
        actor: "test".to_string(),
        source_type: "test".to_string(),
    };

    // Bypass declaration validation to insert a malformed rule
    let bad_class_def = earmark_core::ClassDefinition {
        name: "malformed_rules".to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: earmark_core::JsonSchemaRef("inline:any".to_string()),
        standing_rules: earmark_core::ClassStandingRules::default(),
        relation_rules: vec![
            earmark_core::RelationRule {
                relation_type: "broken".to_string(),
                counterparty_classes: vec!["class_b".to_string()],
                direction: Some("outgoing".to_string()),
                authorizing_endpoint: Some("INVALID_AUTH_MODE".to_string()),
            },
            // Even if we had a valid rule later, the malformed one should fail hard
            earmark_core::RelationRule {
                relation_type: "broken".to_string(),
                counterparty_classes: vec!["class_b".to_string()],
                direction: Some("outgoing".to_string()),
                authorizing_endpoint: Some("source".to_string()),
            },
        ],
        validators: vec![],
    };

    let payload =
        earmark_store::StoredPayload::from_yaml(earmark_core::to_yaml(&bad_class_def).unwrap());
    let stored = earmark_store::StoredObject::new(
        earmark_core::Kind::Object,
        Some("class_definition".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        payload,
        vec![],
    );
    let class_ref = store.write_object(&stored).unwrap();
    index
        .upsert_head_object_from_store(&store, &class_ref.id)
        .unwrap();

    register_class_definition(&store, &index, "class_b", vec![]);

    let obj_a = create_test_object(&store, &index, "malformed_rules");
    let obj_b = create_test_object(&store, &index, "class_b");

    let err = surface
        .create_relation(obj_a, obj_b, "broken".to_string(), json!({}), provenance)
        .unwrap_err();

    assert!(matches!(
        err,
        RuntimeToolError::RelationRuleViolation(ref msg) if msg.contains("unknown authorizing endpoint: INVALID_AUTH_MODE")
    ));
}
