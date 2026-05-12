use assert_cmd::Command;
use earmark_core::{
    Kind, ObjectId, Provenance, Standing, StandingRequestStatus, StandingTransitionRequest,
    VersionId,
};
use earmark_exec::persistence_helpers::write_object_and_index;
use earmark_index::DerivedIndex;
use earmark_store::{GitCanonicalStore, StoredObject, StoredPayload};
use serde_json::Value;
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

fn inject_standing_request(
    root: &std::path::Path,
    status: StandingRequestStatus,
) -> (ObjectId, VersionId) {
    let store = GitCanonicalStore::new(root);
    let index = DerivedIndex::open(root).unwrap();

    let request = StandingTransitionRequest {
        target_object_id: ObjectId::new(),
        dimension: "kernel:epistemic".to_string(),
        from_value: "working".to_string(),
        to_value: "supported".to_string(),
        rationale: Some("Test rationale".to_string()),
        status,
    };

    let payload_bytes = serde_json::to_vec(&request).unwrap();
    let payload = StoredPayload {
        format: earmark_store::PayloadEncoding::Json,
        bytes: payload_bytes,
    };

    let obj = StoredObject::new(
        Kind::Object,
        Some("standing_transition_request".to_string()),
        Standing::default(),
        Provenance::direct_input("test"),
        std::collections::BTreeMap::new(),
        payload,
        vec![],
    );

    write_object_and_index(&store, &index, &obj).unwrap();
    (obj.envelope.id.clone(), obj.envelope.version_id.clone())
}

fn inject_standing_policy(root: &std::path::Path) -> ObjectId {
    let store = GitCanonicalStore::new(root);
    let index = DerivedIndex::open(root).unwrap();

    let policy = earmark_core::StandingPolicy {
        name: "test-policy".to_string(),
        version: "0.1.0".to_string(),
        description: Some("Test policy".to_string()),
        transition_rules: vec![earmark_core::StandingTransitionRule {
            dimension: "kernel:epistemic".to_string(),
            from: vec!["working".to_string()],
            to: vec!["supported".to_string()],
            requires_review: false,
        }],
        operation_requirements: vec![],
        escalations: vec![],
        rationale: Some("Testing".to_string()),
    };

    let payload_bytes = serde_json::to_vec(&policy).unwrap();
    let payload = StoredPayload {
        format: earmark_store::PayloadEncoding::Json,
        bytes: payload_bytes,
    };

    let obj = StoredObject::new(
        Kind::Policy,
        None,
        Standing::default(),
        Provenance::direct_input("test"),
        std::collections::BTreeMap::new(),
        payload,
        vec![],
    );

    write_object_and_index(&store, &index, &obj).unwrap();
    obj.envelope.id.clone()
}

#[test]
fn standing_request_list_outputs_machine_readable_json() {
    let dir = setup_workspace();
    let (id, _) = inject_standing_request(dir.path(), StandingRequestStatus::Proposed);

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("standing-request")
        .arg("list");

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.2.0");
    let requests = parsed["data"].as_array().expect("data should be an array");
    assert!(requests.iter().any(|r| r["id"] == id.as_str()));
}

#[test]
fn standing_request_show_outputs_machine_readable_json() {
    let dir = setup_workspace();
    let (id, _) = inject_standing_request(dir.path(), StandingRequestStatus::Proposed);

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("standing-request")
        .arg("show")
        .arg(id.as_str());

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.2.0");
    assert_eq!(parsed["data"]["ok"], true);
    assert_eq!(parsed["data"]["id"], id.as_str());
    assert_eq!(parsed["data"]["request"]["status"], "proposed");
}

#[test]
fn standing_request_approve_outputs_machine_readable_json() {
    let dir = setup_workspace();
    let (id, _) = inject_standing_request(dir.path(), StandingRequestStatus::Proposed);

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("standing-request")
        .arg("approve")
        .arg(id.as_str())
        .arg("--reason")
        .arg("looks good");

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.2.0");
    assert_eq!(parsed["data"]["ok"], true);
    assert_eq!(parsed["data"]["status"], "approved");
}

#[test]
fn standing_request_reject_outputs_machine_readable_json() {
    let dir = setup_workspace();
    let (id, _) = inject_standing_request(dir.path(), StandingRequestStatus::Proposed);

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("standing-request")
        .arg("reject")
        .arg(id.as_str())
        .arg("--reason")
        .arg("bad request");

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.2.0");
    assert_eq!(parsed["data"]["ok"], true);
    assert_eq!(parsed["data"]["status"], "rejected");
}

#[test]
fn standing_request_apply_outputs_machine_readable_json() {
    let dir = setup_workspace();

    // Applying needs a target object to change its standing
    let store = GitCanonicalStore::new(dir.path());
    let index = DerivedIndex::open(dir.path()).unwrap();
    let target_obj = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("test"),
        std::collections::BTreeMap::new(),
        StoredPayload {
            format: earmark_store::PayloadEncoding::Json,
            bytes: b"{}".to_vec(),
        },
        vec![],
    );
    write_object_and_index(&store, &index, &target_obj).unwrap();
    let target_id = target_obj.envelope.id.clone();

    // Now create an approved request for this target
    let request = StandingTransitionRequest {
        target_object_id: target_id.clone(),
        dimension: "kernel:epistemic".to_string(),
        from_value: "working".to_string(),
        to_value: "supported".to_string(),
        rationale: Some("Test rationale".to_string()),
        status: StandingRequestStatus::Approved,
    };
    let payload_bytes = serde_json::to_vec(&request).unwrap();
    let req_obj = StoredObject::new(
        Kind::Object,
        Some("standing_transition_request".to_string()),
        Standing::default(),
        Provenance::direct_input("test"),
        std::collections::BTreeMap::new(),
        StoredPayload {
            format: earmark_store::PayloadEncoding::Json,
            bytes: payload_bytes,
        },
        vec![],
    );
    write_object_and_index(&store, &index, &req_obj).unwrap();
    let req_id = req_obj.envelope.id.clone();

    let policy_id = inject_standing_policy(dir.path());

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("standing-request")
        .arg("apply")
        .arg(req_id.as_str())
        .arg("--policy")
        .arg(policy_id.as_str())
        .arg("--reason")
        .arg("applying approved change");

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.2.0");
    assert_eq!(parsed["data"]["ok"], true);
    assert_eq!(parsed["data"]["status"], "applied");
    assert_eq!(parsed["data"]["target_id"], target_id.as_str());
}

#[test]
fn standing_request_show_missing_id_outputs_error_envelope() {
    let dir = setup_workspace();

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("standing-request")
        .arg("show")
        .arg("obj_00000000000000000000000000000000");

    let output = cmd.assert().failure().get_output().stderr.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(parsed["contract_version"], "0.2.0");
    assert_eq!(parsed["ok"], false);
    assert!(parsed["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));
}
