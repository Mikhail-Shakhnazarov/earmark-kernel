use assert_cmd::Command;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tempfile::tempdir;

use earmark_core::{Kind, ObjectId, ObjectRef, Provenance, RelationCreationMode, RelationPayload};
use earmark_index::DerivedIndex;
use earmark_store::GitCanonicalStore;

fn workspace_command() -> Command {
    let ws_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.current_dir(ws_root);
    cmd
}

fn setup_and_init_example() -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();

    // 1. em init
    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    // 2. em orchestration init-example
    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("init-example")
        .assert()
        .success();

    (dir, root)
}

fn deposit_object(root: &PathBuf, class: &str, title: &str, payload: &str) -> String {
    deposit_object_with_headers(root, class, title, payload, &[])
}

fn deposit_object_with_headers(
    root: &PathBuf,
    class: &str,
    title: &str,
    payload: &str,
    headers: &[(&str, &str)],
) -> String {
    let mut cmd = workspace_command();
    cmd.arg("--root")
        .arg(root)
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg(class)
        .arg("--title")
        .arg(title);
    for (k, v) in headers {
        cmd.arg("--header").arg(format!("{}={}", k, v));
    }
    cmd.arg("--json-payload").arg(payload);

    let output = cmd.assert().success().get_output().stdout.clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    parsed["data"]["object_id"].as_str().unwrap().to_string()
}

fn link_objects(
    root: &PathBuf,
    source_id: &str,
    source_class: &str,
    target_id: &str,
    target_class: &str,
    relation_type: &str,
) {
    let store = GitCanonicalStore::new(root);
    let mut index = DerivedIndex::open(root).unwrap();

    let source_oid = ObjectId::parse(source_id).unwrap();
    let target_oid = ObjectId::parse(target_id).unwrap();

    let source_vid = index
        .get_head(&source_oid)
        .unwrap()
        .expect("source object version not found in index")
        .version_id;
    let target_vid = index
        .get_head(&target_oid)
        .unwrap()
        .expect("target object version not found in index")
        .version_id;

    let payload = RelationPayload {
        source: ObjectRef::new(
            source_oid,
            source_vid,
            Kind::Object,
            Some(source_class.to_string()),
        ),
        target: ObjectRef::new(
            target_oid,
            target_vid,
            Kind::Object,
            Some(target_class.to_string()),
        ),
        relation_type: relation_type.to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    let mut headers = BTreeMap::new();
    use earmark_core::HeaderValue;
    headers.insert(
        "relation_auth_endpoint".to_string(),
        HeaderValue::String("source".to_string()),
    );
    headers.insert(
        "relation_auth_class".to_string(),
        HeaderValue::String(source_class.to_string()),
    );
    headers.insert(
        "relation_auth_authority".to_string(),
        HeaderValue::String("source".to_string()),
    );
    headers.insert(
        "relation_auth_direction".to_string(),
        HeaderValue::String("outgoing".to_string()),
    );

    earmark_exec::persist_relation_canonical(
        &store,
        &mut index,
        payload,
        Provenance::direct_input("test"),
        RelationCreationMode::Declared,
        Some(headers),
    )
    .unwrap();
}

#[test]
fn test_work_item_show_includes_linked_dispatch_and_context() {
    let (_dir, root) = setup_and_init_example();

    let wi_id = deposit_object_with_headers(
        &root,
        "work_item",
        "WI-1",
        "{\"goal\":\"Show test\",\"status\":\"proposed\",\"priority\":\"medium\"}",
        &[("task_id", "test-task-1")],
    );

    let cp_id = deposit_object(
        &root,
        "context_packet",
        "CP-1",
        "{\"work_item_id\":\"wi_123\",\"instructions\":\"context details\"}",
    );

    let dp_id = deposit_object(
        &root,
        "dispatch",
        "DP-1",
        "{\"work_item_id\":\"wi_123\",\"executor\":\"opencode\"}",
    );

    // Link WI-1 to CP-1 and DP-1
    link_objects(
        &root,
        &wi_id,
        "work_item",
        &cp_id,
        "context_packet",
        "has_context",
    );
    link_objects(
        &root,
        &wi_id,
        "work_item",
        &dp_id,
        "dispatch",
        "has_dispatch",
    );

    // Retrieve via orchestration show
    let show_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("show")
        .arg(&wi_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&show_output).unwrap();
    assert_eq!(parsed["data"]["kind"], "orchestration_work_item_show");
    assert_eq!(parsed["data"]["work_item_id"], wi_id);

    let context_packets = parsed["data"]["context_packets"].as_array().unwrap();
    assert_eq!(context_packets.len(), 1);
    assert_eq!(context_packets[0]["instructions"], "context details");

    let dispatches = parsed["data"]["dispatches"].as_array().unwrap();
    assert_eq!(dispatches.len(), 1);
    assert_eq!(dispatches[0]["executor"], "opencode");
}

#[test]
fn test_timeline_orders_trace_evidence_review_and_closure() {
    let (_dir, root) = setup_and_init_example();

    let wi_id = deposit_object_with_headers(
        &root,
        "work_item",
        "WI-2",
        "{\"goal\":\"Timeline test\",\"status\":\"proposed\",\"priority\":\"medium\"}",
        &[("task_id", "test-task-2")],
    );

    // Wait a brief millisecond/moment to ensure chronological separation of commits
    std::thread::sleep(std::time::Duration::from_millis(50));

    let cp_id = deposit_object(
        &root,
        "context_packet",
        "CP-2",
        "{\"instructions\":\"ctx2\"}",
    );

    std::thread::sleep(std::time::Duration::from_millis(50));

    let dp_id = deposit_object(&root, "dispatch", "DP-2", "{\"executor\":\"opencode\"}");

    std::thread::sleep(std::time::Duration::from_millis(50));

    let te_id = deposit_object(&root, "trace_event", "TE-2", "{\"message\":\"trace2\"}");

    std::thread::sleep(std::time::Duration::from_millis(50));

    let ev_id = deposit_object(&root, "evidence", "EV-2", "{\"checksum\":\"12345\"}");

    std::thread::sleep(std::time::Duration::from_millis(50));

    let rv_id = deposit_object(
        &root,
        "review",
        "RV-2",
        "{\"decision\":\"accepted\",\"comments\":\"perfect\"}",
    );

    std::thread::sleep(std::time::Duration::from_millis(50));

    let cl_id = deposit_object(&root, "closure", "CL-2", "{\"outcome\":\"completed\"}");

    // Establish links
    link_objects(
        &root,
        &wi_id,
        "work_item",
        &cp_id,
        "context_packet",
        "has_context",
    );
    link_objects(
        &root,
        &wi_id,
        "work_item",
        &dp_id,
        "dispatch",
        "has_dispatch",
    );
    link_objects(
        &root,
        &wi_id,
        "work_item",
        &ev_id,
        "evidence",
        "has_evidence",
    );
    link_objects(&root, &wi_id, "work_item", &rv_id, "review", "has_review");
    link_objects(&root, &wi_id, "work_item", &cl_id, "closure", "has_closure");
    link_objects(
        &root,
        &dp_id,
        "dispatch",
        &te_id,
        "trace_event",
        "emitted_trace",
    );

    // Fetch timeline
    let timeline_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("timeline")
        .arg(&wi_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&timeline_output).unwrap();
    assert_eq!(parsed["data"]["kind"], "orchestration_timeline");
    assert_eq!(parsed["data"]["work_item_id"], wi_id);

    let events = parsed["data"]["events"].as_array().unwrap();
    assert_eq!(events.len(), 7);

    // Verify chronological sorting (event indices correspond to order deposited due to sleep timestamps)
    assert_eq!(events[0]["class"], "work_item");
    assert_eq!(events[1]["class"], "context_packet");
    assert_eq!(events[2]["class"], "dispatch");
    assert_eq!(events[3]["class"], "trace_event");
    assert_eq!(events[4]["class"], "evidence");
    assert_eq!(events[5]["class"], "review");
    assert_eq!(events[6]["class"], "closure");
}

#[test]
fn test_index_rebuild_preserves_orchestration_relations() {
    let (_dir, root) = setup_and_init_example();

    let wi_id = deposit_object_with_headers(
        &root,
        "work_item",
        "WI-3",
        "{\"goal\":\"Rebuild test\",\"status\":\"proposed\",\"priority\":\"medium\"}",
        &[("task_id", "test-task-3")],
    );

    let dp_id = deposit_object(&root, "dispatch", "DP-3", "{\"executor\":\"opencode\"}");

    link_objects(
        &root,
        &wi_id,
        "work_item",
        &dp_id,
        "dispatch",
        "has_dispatch",
    );

    // Rebuild index using em doctor --repair-index
    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("doctor")
        .arg("--repair-index")
        .assert()
        .success();

    // Verify the relation remains correctly wired in the rebuilt index
    let show_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("show")
        .arg(&wi_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&show_output).unwrap();
    let dispatches = parsed["data"]["dispatches"].as_array().unwrap();
    assert_eq!(dispatches.len(), 1);
    assert_eq!(dispatches[0]["executor"], "opencode");
}
