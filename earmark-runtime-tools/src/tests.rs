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
