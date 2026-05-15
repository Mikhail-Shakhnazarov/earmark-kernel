use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
use tempfile::tempdir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn test_topology_demo_declarations_validate_and_explain_topology() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let repo_root = workspace_root();

    let system_manifest = repo_root.join("examples/risk-assessment-demo/systems/system.yaml");
    let workflow_manifest = repo_root.join("examples/risk-assessment-demo/workflows/risk_assessment_workflow.yaml");

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
        .arg("declare")
        .arg("register")
        .arg("--kind")
        .arg("system")
        .arg(&system_manifest)
        .assert()
        .success();

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("declare")
        .arg("validate")
        .arg("--kind")
        .arg("workflow")
        .arg(&workflow_manifest)
        .assert()
        .success();

    let explain_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("declare")
        .arg("explain")
        .arg("--kind")
        .arg("workflow")
        .arg(&workflow_manifest)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&explain_output).unwrap();
    assert_eq!(parsed["ok"], true);

    let operations = parsed["data"]["explanation"]["operations"]
        .as_array()
        .unwrap();
    let edges = parsed["data"]["explanation"]["edges"].as_array().unwrap();

    assert!(operations
        .iter()
        .any(|op| op["id"] == "compile_claim_context" && op["kind"] == "compile_context"));
    assert!(operations
        .iter()
        .any(|op| op["id"] == "compile_risk_context" && op["kind"] == "compile_context"));
    assert!(operations
        .iter()
        .any(|op| op["id"] == "compile_assessment_context" && op["kind"] == "compile_context"));
    assert!(edges.iter().any(|e| {
        e["from"] == "compile_assessment_context" && e["to"] == "synthesize_assessment"
    }));
}
