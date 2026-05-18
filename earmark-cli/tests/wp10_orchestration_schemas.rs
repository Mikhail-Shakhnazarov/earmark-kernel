use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

fn workspace_command() -> Command {
    let ws_root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .to_path_buf();
    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.current_dir(ws_root);
    cmd
}

fn setup_and_init_example() -> (tempfile::TempDir, std::path::PathBuf) {
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

#[test]
fn test_create_work_item_valid() {
    let (_dir, root) = setup_and_init_example();

    let output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("work_item")
        .arg("--title")
        .arg("My Work Item")
        .arg("--json-payload")
        .arg("{\"goal\":\"Test goal\",\"status\":\"proposed\",\"priority\":\"medium\"}")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["class"], "work_item");
    assert_eq!(parsed["data"]["title"], "My Work Item");
}

#[test]
fn test_create_work_item_missing_required_title_header() {
    let (_dir, root) = setup_and_init_example();

    // class work_item has required_headers: [title]
    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("work_item")
        .arg("--json-payload")
        .arg("{\"goal\":\"Test goal\",\"status\":\"proposed\",\"priority\":\"medium\"}")
        .assert()
        .failure();
}

#[test]
fn test_create_dispatch_valid() {
    let (_dir, root) = setup_and_init_example();

    let output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("dispatch")
        .arg("--title")
        .arg("Dispatch Alpha")
        .arg("--json-payload")
        .arg("{\"work_item_id\":\"wi_123\",\"status\":\"queued\"}")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["class"], "dispatch");
}

#[test]
fn test_create_context_packet_valid() {
    let (_dir, root) = setup_and_init_example();

    let output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("context_packet")
        .arg("--title")
        .arg("Context Packet Alpha")
        .arg("--json-payload")
        .arg("{\"work_item_id\":\"wi_123\",\"instructions\":\"Run smoke tests\"}")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["class"], "context_packet");
}

#[test]
fn test_create_trace_event_valid() {
    let (_dir, root) = setup_and_init_example();

    let output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("trace_event")
        .arg("--title")
        .arg("Trace Step")
        .arg("--json-payload")
        .arg(
            "{\"work_item_id\":\"wi_123\",\"event_type\":\"started\",\"summary\":\"Started task\"}",
        )
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["class"], "trace_event");
}

#[test]
fn test_create_evidence_valid() {
    let (_dir, root) = setup_and_init_example();

    let output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("evidence")
        .arg("--title")
        .arg("Evidence Alpha")
        .arg("--json-payload")
        .arg("{\"work_item_id\":\"wi_123\",\"evidence_type\":\"command_output\",\"summary\":\"Tests passed\"}")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["class"], "evidence");
}

#[test]
fn test_create_review_valid() {
    let (_dir, root) = setup_and_init_example();

    let output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("review")
        .arg("--title")
        .arg("Review Alpha")
        .arg("--json-payload")
        .arg("{\"work_item_id\":\"wi_123\",\"verdict\":\"approved\"}")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["class"], "review");
}

#[test]
fn test_create_closure_valid() {
    let (_dir, root) = setup_and_init_example();

    let output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("closure")
        .arg("--title")
        .arg("Closure Alpha")
        .arg("--json-payload")
        .arg("{\"work_item_id\":\"wi_123\",\"disposition\":\"completed\",\"summary\":\"Done\"}")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["class"], "closure");
}
