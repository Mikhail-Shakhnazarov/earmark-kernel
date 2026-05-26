use assert_cmd::Command;
use serde_json::Value;
use std::fs;
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
fn test_json_provider_capabilities_include_plugin_aliases() {
    let dir = setup_workspace();
    let plugin_dir = dir
        .path()
        .join(".earmark")
        .join("plugins")
        .join("providers");
    fs::create_dir_all(&plugin_dir).unwrap();
    fs::write(
        plugin_dir.join("openai_http.yaml"),
        r#"
schema: earmark.provider_plugin.v1
name: openai-http
version: 0.1.0
providers:
  - provider: openai_compatible_http
    adapter: http_generation
    required_env:
      - OPENAI_API_KEY
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("provider")
        .arg("capabilities");

    let output = cmd.assert().success().get_output().stdout.clone();
    let val = verify_envelope(&output);
    let providers = val["data"]["providers"]
        .as_array()
        .expect("providers should be an array");
    let alias = providers
        .iter()
        .find(|provider| provider["provider"] == "openai_compatible_http")
        .expect("plugin alias should be listed");
    assert_eq!(alias["status"], "missing_configuration");
    assert!(val["data"]["loaded_provider_plugins"]
        .as_array()
        .expect("loaded_provider_plugins should be an array")
        .iter()
        .any(|plugin| plugin["name"] == "openai-http"));
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
