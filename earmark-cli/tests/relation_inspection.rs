use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn relation_inspection_and_explanation() {
    let dir = tempdir().unwrap();
    let workspace = workspace_root();

    // 1. Init
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("init")
        .assert()
        .success();

    // 2. Register classes
    let class_path = workspace.join("docs/declarations/examples/classes/finding.yaml");
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("declare")
        .arg("register")
        .arg("--kind")
        .arg("class")
        .arg(&class_path)
        .assert()
        .success();

    // 3. Deposit objects
    let dep1 = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("finding")
        .arg("--title")
        .arg("Finding 1")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id1 = serde_json::from_slice::<Value>(&dep1).unwrap()["data"]["object_id"]
        .as_str()
        .unwrap()
        .to_string();

    let dep2 = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("source_note")
        .arg("--title")
        .arg("Source 1")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id2 = serde_json::from_slice::<Value>(&dep2).unwrap()["data"]["object_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 4. Create relation using deposit kind=relation (this is a bit of a hack but it works if implemented)
    // Actually, I don't have a direct "create relation" CLI command yet, 
    // but the task was to update inspection/explanation.
    // I'll use a trick: register a system and run a workflow that creates a relation.
    // Or I can use `em deposit` with `--kind relation`.
    
    // Let's see if `em deposit` handles relations.
    // In `deposit.rs`:
    // let payload_value = if args.json_payload.is_some() || kind == Kind::Relation { ... }
    
    let rel_payload = serde_json::json!({
        "source": { "id": id1, "version_id": "ver_00000000000000000000000000000000", "kind": "object", "class": "finding" },
        "target": { "id": id2, "version_id": "ver_00000000000000000000000000000000", "kind": "object", "class": "source_note" },
        "relation_type": "derived_from",
        "qualifiers": {},
    });

    let dep_rel = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("finding")
        .arg("--kind")
        .arg("relation")
        .arg("--json-payload")
        .arg(serde_json::to_string(&rel_payload).unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let rel_id = serde_json::from_slice::<Value>(&dep_rel).unwrap()["data"]["object_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 5. Test em relation show
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("relation")
        .arg("show")
        .arg(&rel_id)
        .assert()
        .success();

    // 6. Test em relation explain
    let explain_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("relation")
        .arg("explain")
        .arg(&rel_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let explain_text = String::from_utf8(explain_output).unwrap();
    assert!(explain_text.contains("RELATION Explanation"));
    assert!(explain_text.contains("derived_from"));

    // 7. Test em relation list
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("relation")
        .arg("list")
        .arg("--source-id")
        .arg(&id1)
        .assert()
        .success();
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
