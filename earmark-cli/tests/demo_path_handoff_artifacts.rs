use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn run_artifacts_exposes_handoffs_for_demo_path() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let repo_root = workspace_root();

    let system_manifest = repo_root.join("examples/research-synthesis/declarations/systems/system.yaml");
    let seed_note = repo_root.join("examples/research-synthesis/data/seed_notes/note_1_benefits.md");

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("system")
        .arg("register")
        .arg(&system_manifest)
        .assert()
        .success();

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

    let deposit_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("source_note")
        .arg("--title")
        .arg("Federated Graphs: Agility and Ownership")
        .arg("--payload-file")
        .arg(&seed_note)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&deposit_output).unwrap();
    let note_id = parsed["data"]["object_id"].as_str().unwrap().to_string();

    let run_output = Command::cargo_bin("earmark-cli")
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
        .arg(&note_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&run_output).unwrap();
    assert_eq!(parsed["data"]["ok"], true);
    assert_eq!(parsed["data"]["status"], "completed");
    let run_id = parsed["data"]["run_id"].as_str().unwrap().to_string();

    let artifacts_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("run")
        .arg("artifacts")
        .arg(&run_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&artifacts_output).unwrap();
    assert_eq!(parsed["data"]["ok"], true);

    let artifact = &parsed["data"]["artifact"];
    assert!(
        artifact.get("handoff_ids").is_none(),
        "run artifacts should expose handoffs under the current `handoffs` key, not the stale `handoff_ids` key"
    );
    let handoffs = artifact["handoffs"]
        .as_array()
        .expect("run artifacts should contain a handoffs array");
    assert!(
        !handoffs.is_empty(),
        "demo workflow should create at least one handoff"
    );

    let first_handoff = handoffs[0]
        .as_str()
        .expect("handoff entries should be handoff IDs");
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("handoff")
        .arg("explain")
        .arg(first_handoff)
        .assert()
        .success();
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
