use assert_cmd::Command;
use chrono::Utc;
use earmark_core::{Kind, ObjectId, Provenance, Standing, VersionId, VersionRef};
use earmark_exec::persistence_helpers::write_object_and_index;
use earmark_index::DerivedIndex;
use earmark_store::{GitCanonicalStore, StoredObject, StoredPayload};
use serde_json::Value;
use std::collections::BTreeMap;
use tempfile::tempdir;

fn setup_workspace() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("init")
        .assert()
        .success();
    dir
}

fn make_run_record(run_id: &str, started_at: chrono::DateTime<Utc>) -> earmark_core::RunRecord {
    earmark_core::RunRecord {
        run_id: run_id.to_string(),
        system_definition: VersionRef::new(ObjectId::new(), VersionId::new()),
        workflow: VersionRef::new(ObjectId::new(), VersionId::new()),
        status: earmark_core::RunStatus::Completed,
        started_at,
        ended_at: None,
        initial_marking: vec![],
        final_marking: vec![],
        events: vec![],
        work_packets: vec![],
        governance_events: vec![],
        assignments: vec![],
        change_sets: vec![],
        manifests: vec![],
    }
}

fn write_run(store: &GitCanonicalStore, index: &DerivedIndex, record: &earmark_core::RunRecord) {
    let obj = StoredObject::new(
        Kind::RunRecord,
        None,
        Standing::default(),
        Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(record).unwrap()),
        vec![],
    );
    write_object_and_index(store, index, &obj).unwrap();
}

#[test]
fn list_run_records_deterministic_order() {
    let dir = setup_workspace();
    let store = GitCanonicalStore::new(dir.path());
    let index = DerivedIndex::open(dir.path()).unwrap();

    let now = Utc::now();

    let records = [
        make_run_record("obj_a", now),
        make_run_record("obj_b", now),
        make_run_record("obj_c", now),
    ];
    for rec in &records {
        write_run(&store, &index, rec);
    }

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("list");
    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    let runs = parsed["data"]["runs"].as_array().unwrap();

    assert_eq!(runs.len(), 3);
    assert_eq!(runs[0]["run_id"], "obj_a", "first run should be obj_a (lexicographic tie-break)");
    assert_eq!(runs[1]["run_id"], "obj_b", "second run should be obj_b (lexicographic tie-break)");
    assert_eq!(runs[2]["run_id"], "obj_c", "third run should be obj_c (lexicographic tie-break)");
}

#[test]
fn latest_resolves_to_last_lexicographically_when_timestamps_equal() {
    let dir = setup_workspace();
    let store = GitCanonicalStore::new(dir.path());
    let index = DerivedIndex::open(dir.path()).unwrap();

    let now = Utc::now();

    let records = [
        make_run_record("obj_a_earliest", now),
        make_run_record("obj_z_latest", now),
    ];
    for rec in &records {
        write_run(&store, &index, rec);
    }

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("show")
        .arg("latest");
    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(
        parsed["data"]["run_id"], "obj_z_latest",
        "latest should pick obj_z_latest (lexicographically greater at same timestamp)"
    );
}

#[test]
fn latest_respects_temporal_order_when_timestamps_differ() {
    let dir = setup_workspace();
    let store = GitCanonicalStore::new(dir.path());
    let index = DerivedIndex::open(dir.path()).unwrap();

    let now = Utc::now();
    let later = now + chrono::Duration::seconds(10);

    let records = [
        make_run_record("obj_older", now),
        make_run_record("obj_newer", later),
    ];
    for rec in &records {
        write_run(&store, &index, rec);
    }

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("show")
        .arg("latest");
    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(
        parsed["data"]["run_id"], "obj_newer",
        "latest should pick obj_newer (later timestamp)"
    );
}

#[test]
fn list_order_respects_temporal_then_lexicographic() {
    let dir = setup_workspace();
    let store = GitCanonicalStore::new(dir.path());
    let index = DerivedIndex::open(dir.path()).unwrap();

    let early = Utc::now();
    let late = early + chrono::Duration::hours(1);

    let records = [
        make_run_record("obj_z_early", early),
        make_run_record("obj_a_early", early),
        make_run_record("obj_m_late", late),
    ];
    for rec in &records {
        write_run(&store, &index, rec);
    }

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("list");
    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    let runs = parsed["data"]["runs"].as_array().unwrap();

    assert_eq!(runs.len(), 3);
    assert_eq!(
        runs[0]["run_id"], "obj_a_early",
        "first: obj_a_early (earliest time, then lexicographic)"
    );
    assert_eq!(
        runs[1]["run_id"], "obj_z_early",
        "second: obj_z_early (earliest time, then lexicographic)"
    );
    assert_eq!(
        runs[2]["run_id"], "obj_m_late",
        "third: obj_m_late (later time)"
    );
}
