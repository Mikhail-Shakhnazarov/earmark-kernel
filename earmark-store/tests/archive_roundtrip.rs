/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use chrono::Utc;
use earmark_core::*;
use earmark_declarations::InProcessRegistry;
use earmark_store::archive::{export_workspace, import_workspace};
use earmark_store::file_store::FileStore;
use earmark_store::sanctioned::{deposit_object, DepositObjectInput};
use earmark_store::traits::CanonicalStore;
use tempfile::tempdir;

#[test]
fn archive_roundtrip_preserves_readable_store_records() {
    let source_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();

    let source = FileStore::new(source_dir.path());
    let target = FileStore::new(target_dir.path());

    source.init().unwrap();
    target.init().unwrap();

    let registry = InProcessRegistry::new();
    let actor = ActorId::generate();

    let object = deposit_object(
        &source,
        &registry,
        actor.clone(),
        DepositObjectInput {
            id: None,
            class_id: None,
            payload: serde_json::json!({"title": "roundtrip"}),
            standing: Standing { dimensions: vec![] },
            signal: None,
        },
    )
    .unwrap();

    let target_object = deposit_object(
        &source,
        &registry,
        actor.clone(),
        DepositObjectInput {
            id: None,
            class_id: None,
            payload: serde_json::json!({"title": "target"}),
            standing: Standing { dimensions: vec![] },
            signal: None,
        },
    )
    .unwrap();

    let relation = RelationRecord {
        id: RelationId::generate(),
        source_id: object.id.clone(),
        target_id: target_object.id.clone(),
        relation_type: "depends_on".to_string(),
        created_at: Utc::now(),
        created_by: Some(actor),
    };

    source.create_relation(relation.clone()).unwrap();

    let archive = export_workspace(&source, false).unwrap();
    import_workspace(&target, archive, false).unwrap();

    let restored_object = target.get_object(&object.id).unwrap();
    assert_eq!(restored_object.id, object.id);

    let restored_relation = target.get_relation(&relation.id).unwrap();
    assert_eq!(restored_relation.source_id, object.id);

    let violations = target.verify_regression_gate().unwrap();
    assert!(violations.is_empty(), "round-tripped store has violations: {violations:?}");
}
