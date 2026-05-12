use assert_cmd::Command;
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

fn verify_envelope(stdout: &[u8]) -> Value {
    let val: Value = serde_json::from_slice(stdout).expect("valid JSON output");
    assert_eq!(val["contract_version"], "0.2.0");
    assert!(val.get("data").is_some() || !val["ok"].as_bool().unwrap_or(true));
    val
}

#[test]
fn test_json_status() {
    let dir = setup_workspace();
    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("status");

    let output = cmd.assert().success().get_output().stdout.clone();
    let val = verify_envelope(&output);
    let data = &val["data"];
    assert!(data.get("object_count").is_some());
    assert!(data.get("active_system_count").is_some());
}

#[test]
fn test_json_run_list() {
    let dir = setup_workspace();
    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("list");

    let output = cmd.assert().success().get_output().stdout.clone();
    let val = verify_envelope(&output);
    assert!(val["data"]["runs"].is_array());
}

#[test]
fn test_json_provider_capabilities() {
    let dir = setup_workspace();
    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("provider")
        .arg("capabilities");

    let output = cmd.assert().success().get_output().stdout.clone();
    let val = verify_envelope(&output);
    assert!(val["data"]["providers"].is_array());
}

#[test]
fn test_json_standing_request_list() {
    let dir = setup_workspace();
    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("standing-request")
        .arg("list");

    let output = cmd.assert().success().get_output().stdout.clone();
    let val = verify_envelope(&output);
    // Let's see what standing-request list returns
    assert!(val["data"].is_array());
}
