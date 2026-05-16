use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

fn verify_error_envelope(output: &[u8]) -> Value {
    let val: Value = serde_json::from_slice(output).expect("valid JSON error envelope");
    assert_eq!(val["contract_version"], "0.2.0");
    assert_eq!(val["ok"], false);
    assert!(val["error"]["message"].as_str().is_some());
    val
}

#[test]
fn test_invalid_workspace() {
    let dir = tempdir().unwrap();
    let bad_path = dir.path().join("does_not_exist");

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    let output = cmd
        .arg("--root")
        .arg(&bad_path)
        .arg("--json")
        .arg("status")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let val = verify_error_envelope(&output);
    assert!(val["error"]["message"].as_str().unwrap().contains("not initialized")
        || val["error"]["message"].as_str().unwrap().contains("not found")
        || val["error"]["message"].as_str().unwrap().contains("No such"));
}

#[test]
fn test_malformed_yaml() {
    let dir = tempdir().unwrap();

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    let yaml_path = dir.path().join("bad.yaml");
    std::fs::write(&yaml_path, "[invalid yaml: {").unwrap();

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    let output = cmd
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("declare")
        .arg("validate")
        .arg("--kind")
        .arg("class")
        .arg(&yaml_path)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let val = verify_error_envelope(&output);
    assert!(val["error"]["message"].as_str().unwrap().to_lowercase().contains("yaml"));
}

#[test]
fn test_non_existent_object() {
    let dir = tempdir().unwrap();

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    let output = cmd
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("relation")
        .arg("show")
        .arg("obj_00000000000000000000000000000000")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let val = verify_error_envelope(&output);
    assert!(val["error"]["message"].as_str().unwrap().contains("not found")
        || val["error"]["message"].as_str().unwrap().contains("404"));
}

#[test]
fn test_non_existent_run() {
    let dir = tempdir().unwrap();

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    let output = cmd
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("show")
        .arg("nonexistent_run")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let val = verify_error_envelope(&output);
    assert!(val["error"]["message"].as_str().unwrap().contains("not found")
        || val["error"]["message"].as_str().unwrap().contains("404"));
}
