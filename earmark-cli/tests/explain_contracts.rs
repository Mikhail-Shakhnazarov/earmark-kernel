use assert_cmd::Command;
use chrono::Utc;
use earmark_core::{Kind, ObjectId, ObjectRef, Provenance, Standing, VersionId, VersionRef};
use earmark_exec::persistence_helpers::write_object_and_index;
use earmark_index::DerivedIndex;
use earmark_store::{GitCanonicalStore, StoredObject, StoredPayload};
use serde_json::Value;
use std::collections::BTreeMap;
use tempfile::tempdir;

fn setup_workspace() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    Command::cargo_bin("em")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("init")
        .assert()
        .success();
    dir
}

#[test]
fn declare_explain_contracts() {
    let dir = setup_workspace();
    let class_path = dir.path().join("test_class.yaml");
    std::fs::write(&class_path, "name: test_class\nversion: 0.1.0\nkind: object\nrequired_headers: []\npayload_schema: null\nstanding_rules: {}\nrelation_rules: []\nvalidators: []").unwrap();

    let mut cmd = Command::cargo_bin("em").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("declare")
        .arg("explain")
        .arg("--kind")
        .arg("class")
        .arg(class_path);

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.3.0");
    assert_eq!(parsed["ok"], true);
    assert!(parsed["data"]["explanation"]["title"].is_string());
}

#[test]
fn run_explain_contracts() {
    let dir = setup_workspace();
    let store = GitCanonicalStore::new(dir.path());
    let mut index = DerivedIndex::open(dir.path()).unwrap();

    let run_record = earmark_core::RunRecord {
        run_id: earmark_core::RunId::parse("test_run").unwrap(),
        system_definition: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        workflow: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        status: earmark_core::RunStatus::Running,
        started_at: Utc::now(),
        ended_at: None,
        initial_marking: vec![],
        final_marking: vec![],
        events: vec![],
        work_packets: vec![],
        governance_events: vec![],
        assignments: vec![],
        change_sets: vec![],
        manifests: vec![],
    };

    let obj = StoredObject::new(
        Kind::RunRecord,
        None,
        Standing::default(),
        Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&run_record).unwrap()),
        vec![],
    );

    write_object_and_index(&store, &mut index, &obj).unwrap();

    let mut cmd = Command::cargo_bin("em").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("explain")
        .arg("test_run");

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.3.0");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "run");
    assert!(parsed["data"]["summary"].is_string());
    assert!(parsed["data"]["related"].is_object());
}

#[test]
fn assignment_explain_contracts() {
    let dir = setup_workspace();
    let store = GitCanonicalStore::new(dir.path());
    let mut index = DerivedIndex::open(dir.path()).unwrap();

    let assignment = earmark_core::TransitionAssignment {
        id: earmark_core::TransitionAssignmentId::generate(),
        run_id: earmark_core::RunId::parse("test_run").unwrap(),
        transition_id: earmark_core::TransitionId::parse("t1").unwrap(),
        assigned_to: "agent_1".to_string(),
        status: earmark_core::AssignmentStatus::Assigned,
        input_object_ids: vec![],
        handoff_manifest_id: None,
        event_ids: vec![],
        blocked_reason: None,
        completion_change_set_id: None,
        assigned_at: Utc::now(),
        updated_at: Utc::now(),
        expires_at: None,
        completed_at: None,
    };

    let obj = StoredObject::new(
        Kind::TransitionAssignment,
        None,
        Standing::default(),
        Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&assignment).unwrap()),
        vec![],
    );

    write_object_and_index(&store, &mut index, &obj).unwrap();

    let mut cmd = Command::cargo_bin("em").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("assignment")
        .arg("explain")
        .arg(assignment.id.as_str());

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.3.0");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "assignment");
    assert!(parsed["data"]["summary"].is_string());
    assert!(parsed["data"]["related"].is_object());
}

#[test]
fn changeset_explain_contracts() {
    let dir = setup_workspace();
    let store = GitCanonicalStore::new(dir.path());
    let mut index = DerivedIndex::open(dir.path()).unwrap();

    let changeset = earmark_core::ChangeSet {
        id: earmark_core::ChangeSetId::generate(),
        run_id: earmark_core::RunId::parse("test_run").unwrap(),
        transition_id: earmark_core::TransitionId::parse("t1").unwrap(),
        assignment_id: Some(earmark_core::TransitionAssignmentId::generate()),
        agent_id: None,
        input_object_ids: vec![],
        created_object_ids: vec![],
        created_relation_ids: vec![],
        updated_object_ids: vec![],
        governance_event_ids: vec![],
        blocked_operations: vec![],
        unresolved_ambiguities: vec![],
        rejected_candidates: vec![],
        validation_results: vec![],
        work_packet_id: None,
        handoff_manifest_id: None,
        created_at: Utc::now(),
    };

    let obj = StoredObject::new(
        Kind::ChangeSet,
        None,
        Standing::default(),
        Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&changeset).unwrap()),
        vec![],
    );

    write_object_and_index(&store, &mut index, &obj).unwrap();

    let mut cmd = Command::cargo_bin("em").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("change-set")
        .arg("explain")
        .arg(changeset.id.as_str());

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.3.0");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "change_set");
    assert!(parsed["data"]["summary"].is_string());
    assert!(parsed["data"]["related"].is_object());
}

#[test]
fn handoff_explain_contracts() {
    let dir = setup_workspace();
    let store = GitCanonicalStore::new(dir.path());
    let mut index = DerivedIndex::open(dir.path()).unwrap();

    let handoff = earmark_core::HandoffManifest {
        id: earmark_core::HandoffManifestId::generate(),
        run_id: earmark_core::RunId::parse("test_run").unwrap(),
        from_transition_id: earmark_core::TransitionId::parse("t1").unwrap(),
        to_transition_id: Some(earmark_core::TransitionId::parse("t2").unwrap()),
        source_change_set_id: earmark_core::ChangeSetId::generate(),
        source_assignment_id: None,
        root_object_ids: vec![],
        inherited_input_object_ids: vec![],
        newly_created_object_ids: vec![],
        newly_created_relation_ids: vec![],
        allowed_input_classes: vec![],
        allowed_output_classes: vec![],
        allowed_relation_types: vec![],
        standing_constraints: vec![],
        unresolved_ambiguities: vec![],
        blocked_conditions: vec![],
        required_checks: vec![],
        compiled_context_template_id: None,
        created_at: Utc::now(),
    };

    let obj = StoredObject::new(
        Kind::HandoffManifest,
        None,
        Standing::default(),
        Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&handoff).unwrap()),
        vec![],
    );

    write_object_and_index(&store, &mut index, &obj).unwrap();

    let mut cmd = Command::cargo_bin("em").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("handoff")
        .arg("explain")
        .arg(handoff.id.as_str());

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.3.0");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "handoff");
    assert!(parsed["data"]["summary"].is_string());
    assert!(parsed["data"]["related"].is_object());
}

#[test]
fn failure_explain_contracts() {
    let dir = setup_workspace();
    let store = GitCanonicalStore::new(dir.path());
    let mut index = DerivedIndex::open(dir.path()).unwrap();

    let failure = earmark_core::TransformationFailure {
        run_id: earmark_core::RunId::parse("test_run").unwrap(),
        transition_id: earmark_core::TransitionId::parse("t1").unwrap(),
        assignment_id: earmark_core::TransitionAssignmentId::generate(),
        failed_change_set_id: None,
        error_type: "test_error".to_string(),
        message: "something went wrong".to_string(),
        stack_trace: None,
        input_object_ids: vec![],
        created_at: Utc::now(),
    };

    let obj = StoredObject::new(
        Kind::TransformationFailure,
        None,
        Standing::default(),
        Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&failure).unwrap()),
        vec![],
    );

    write_object_and_index(&store, &mut index, &obj).unwrap();

    let mut cmd = Command::cargo_bin("em").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("failure")
        .arg("explain")
        .arg(obj.envelope.id.as_str());

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.3.0");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "failure");
    assert!(parsed["data"]["summary"].is_string());
    assert!(parsed["data"]["related"].is_object());
}

#[test]
fn relation_explain_contracts() {
    let dir = setup_workspace();
    let store = GitCanonicalStore::new(dir.path());
    let mut index = DerivedIndex::open(dir.path()).unwrap();

    let relation = earmark_core::RelationPayload {
        source: ObjectRef::new(
            ObjectId::generate(),
            VersionId::generate(),
            Kind::Object,
            None,
        ),
        target: ObjectRef::new(
            ObjectId::generate(),
            VersionId::generate(),
            Kind::Object,
            None,
        ),
        relation_type: "test_relation".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    let obj = StoredObject::new(
        Kind::Relation,
        None,
        Standing::default(),
        Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&relation).unwrap()),
        vec![],
    );

    write_object_and_index(&store, &mut index, &obj).unwrap();

    let mut cmd = Command::cargo_bin("em").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("relation")
        .arg("explain")
        .arg(obj.envelope.id.as_str());

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.3.0");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "relation");
    assert!(parsed["data"]["summary"].is_string());
    assert!(parsed["data"]["related"].is_object());
}
