use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_quickstart_smoke_path() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let workspace = workspace_root();

    // 1. em init
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    // 2. em system register
    let manifest = workspace.join("examples/research-synthesis/declarations/systems/system.yaml");
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("system")
        .arg("register")
        .arg(&manifest)
        .assert()
        .success();

    // 3. em system activate
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("system")
        .arg("activate")
        .arg("sys_research_synthesis")
        .assert()
        .success();

    // 4. em deposit
    let deposit_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("source_note")
        .arg("--title")
        .arg("Smoke Test Note")
        .arg("--body")
        .arg("This is a smoke test.")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&deposit_output).unwrap();
    let object_id = parsed["data"]["object_id"]
        .as_str()
        .expect("object_id missing");

    // 5. em query
    let query_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("query")
        .arg("--class")
        .arg("source_note")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_query: Value = serde_json::from_slice(&query_output).unwrap();
    let items = parsed_query["data"]
        .as_array()
        .expect("query data not an array");
    assert!(!items.is_empty());
    assert_eq!(items[0]["object_id"], object_id);

    // 6. em workflow run
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("workflow")
        .arg("run")
        .arg("research_synthesis")
        .arg("--system-id")
        .arg("sys_research_synthesis")
        .arg("--with")
        .arg(object_id)
        .assert()
        .success();

    // 7. em run explain latest
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("run")
        .arg("explain")
        .arg("latest")
        .assert()
        .success();
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
