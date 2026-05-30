/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use crate::errors::StoreError;
use crate::traits::CanonicalStore;
use chrono::Utc;
use earmark_core::WorkspaceArchive;

pub fn export_workspace(
    store: &dyn CanonicalStore,
    redact: bool,
) -> Result<WorkspaceArchive, StoreError> {
    let mut objects = Vec::new();
    for oid in store.list_objects()? {
        let obj = store.get_object(&oid)?;
        let mut versions = Vec::new();
        for vid in store.list_versions(&oid)? {
            let mut version = store.get_version(&oid, &vid)?;
            if redact {
                version.payload = serde_json::Value::Null;
                if let Some(ref mut signal) = version.signal {
                    signal.integrity_state = earmark_core::SignalIntegrityState::Unchecked;
                }
            }
            versions.push(version);
        }
        objects.push((obj, versions));
    }

    let mut relations = Vec::new();
    for rid in store.list_all_relations()? {
        relations.push(store.get_relation(&rid)?);
    }

    let mut runs = Vec::new();
    for rid in store.list_runs()? {
        runs.push(store.get_run(&rid)?);
    }

    let mut packets = Vec::new();
    for pid in store.list_packets()? {
        packets.push(store.get_packet(&pid)?);
    }

    let mut dispatches = Vec::new();
    for did in store.list_dispatches()? {
        dispatches.push(store.get_dispatch(&did)?);
    }

    let mut change_sets = Vec::new();
    // Assuming list_change_sets exists or we just collect them all if possible
    // For now, change_set support might be limited in the store trait

    let mut handoffs = Vec::new();
    // Similar for handoffs

    let mut standing = Vec::new();
    for oid in store.list_objects()? {
        let st = store.get_standing(&earmark_core::StandingTargetRef::Object(oid))?;
        standing.extend(st);
    }

    let mut system_packs = Vec::new();
    for spid in store.list_system_packs()? {
        system_packs.push(store.get_system_pack(&spid)?);
    }

    let mut reviews = Vec::new();
    for rid in store.list_reviews()? {
        reviews.push(store.get_review(&rid)?);
    }

    let mut classes = Vec::new();
    for cid in store.list_classes()? {
        classes.push(store.get_class(&cid)?);
    }

    let mut systems = Vec::new();
    for sid in store.list_systems()? {
        systems.push(store.get_system(&sid)?);
    }

    let mut workflows = Vec::new();
    for wid in store.list_workflows()? {
        workflows.push(store.get_workflow(&wid)?);
    }

    let archive = WorkspaceArchive {
        objects,
        relations,
        runs,
        packets,
        dispatches,
        change_sets,
        handoffs,
        reviews,
        standing,
        system_packs,
        classes,
        systems,
        workflows,
        migrations: store.get_migration_history()?,
        exported_at: Utc::now(),
        protocol_version: "0.1.0".to_string(),
    };

    Ok(archive)
}

pub fn import_workspace(
    store: &dyn CanonicalStore,
    archive: WorkspaceArchive,
    overwrite: bool,
) -> Result<(), StoreError> {
    store.import_archive(archive, overwrite)
}
