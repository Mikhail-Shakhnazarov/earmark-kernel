use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

fn workspace_command() -> Command {
    Command::cargo_bin("earmark-cli").unwrap()
}

#[test]
fn test_provider_list_json_schema() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Ensure .earmark exists
    fs::create_dir_all(root.join(".earmark")).unwrap();

    let output = workspace_command()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("provider")
        .arg("list")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let val: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(val["data"]["kind"], "provider_list");
    assert!(val["data"]["search_paths"].is_array());
    assert!(val["data"]["loaded_plugins"].is_array());

    // Verify default search path is included
    let search_paths = val["data"]["search_paths"].as_array().unwrap();
    assert!(search_paths
        .iter()
        .any(|p| p.as_str().unwrap().contains(".earmark/plugins/providers")));
}

#[test]
fn test_provider_list_env_dirs_are_included() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let extra_dir = dir.path().join("extra_plugins");
    fs::create_dir_all(&extra_dir).unwrap();

    let output = workspace_command()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .env("EM_PROVIDER_PLUGIN_DIRS", extra_dir.to_str().unwrap())
        .arg("provider")
        .arg("list")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let val: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let search_paths = val["data"]["search_paths"].as_array().unwrap();
    assert!(search_paths
        .iter()
        .any(|p| p.as_str().unwrap().contains("extra_plugins")));
}
