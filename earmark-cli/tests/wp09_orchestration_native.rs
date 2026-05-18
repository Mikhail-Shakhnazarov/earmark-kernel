use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_orchestration_ingest_source_validation() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // 1. em init
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    // 2. Try to ingest task with invalid source
    let assert_res = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("orchestration")
        .arg("ingest-task")
        .arg("--source")
        .arg("invalid-source-name")
        .arg("some-task-id")
        .assert()
        .failure();

    let err_msg = String::from_utf8_lossy(&assert_res.get_output().stdout);
    assert!(err_msg.contains("unsupported source"), "Error message did not complain about source");
    assert!(err_msg.contains("Supported sources"), "Error message did not list supported sources");
}

#[test]
fn test_orchestration_ingest_native_json_single() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // 1. em init
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    // 2. Create a temporary native JSON work item file
    let json_path = dir.path().join("task.json");
    let payload = serde_json::json!({
        "title": "Build native orchestration ledger",
        "goal": "Implement native Earmark work_item, evidence, and review",
        "priority": "high",
        "status": "proposed"
    });
    fs::write(&json_path, serde_json::to_string(&payload).unwrap()).unwrap();

    // 3. Ingest using native-json source
    let ingest_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("orchestration")
        .arg("ingest-task")
        .arg("--source")
        .arg("native-json")
        .arg(json_path.to_str().unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&ingest_output).unwrap();
    assert_eq!(parsed["data"]["source"], "native-json");
    let tasks = parsed["data"]["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "Build native orchestration ledger");
    assert_eq!(tasks[0]["status"], "proposed");
    assert_eq!(tasks[0]["priority"], "high");
}

#[test]
fn test_orchestration_ingest_native_json_batch() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // 1. em init
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    // 2. Create a temporary batch JSON file
    let json_path = dir.path().join("batch.json");
    let payload = serde_json::json!({
        "schema": "earmark.orchestration.ingest.v1",
        "records": [
            {
                "kind": "orchestration.work_item.v1",
                "title": "Subtask A",
                "goal": "Verify WP1 boundary",
                "priority": "high",
                "status": "proposed"
            },
            {
                "kind": "orchestration.work_item.v1",
                "title": "Subtask B",
                "goal": "Verify WP2 schemas",
                "priority": "medium",
                "status": "proposed"
            }
        ]
    });
    fs::write(&json_path, serde_json::to_string(&payload).unwrap()).unwrap();

    // 3. Ingest using local-json source
    let ingest_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("orchestration")
        .arg("ingest-task")
        .arg("--source")
        .arg("local-json")
        .arg(json_path.to_str().unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&ingest_output).unwrap();
    let tasks = parsed["data"]["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0]["title"], "Subtask A");
    assert_eq!(tasks[1]["title"], "Subtask B");
}
