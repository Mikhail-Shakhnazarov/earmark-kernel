use assert_cmd::Command;
use serde_json::Value;
use std::collections::HashMap;
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
    let workflow_manifest =
        repo_root.join("examples/risk-assessment-demo/workflows/risk_assessment_workflow.yaml");

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
    let guards = parsed["data"]["explanation"]["guards"].as_array().unwrap();

    assert!(
        !guards.is_empty(),
        "workflow should declare at least one guard"
    );
    assert!(
        edges.iter().any(|e| e["condition"].is_string()),
        "workflow should include at least one conditional edge"
    );

    assert!(operations
        .iter()
        .any(|op| op["id"] == "compile_claim_context" && op["kind"] == "compile_context"));
    assert!(operations
        .iter()
        .any(|op| op["id"] == "compile_risk_context" && op["kind"] == "compile_context"));
    assert!(operations
        .iter()
        .any(|op| op["id"] == "compile_assessment_context" && op["kind"] == "compile_context"));
    assert!(operations
        .iter()
        .any(|op| op["id"] == "review_assessment" && op["kind"] == "review"));
    assert!(operations
        .iter()
        .any(|op| op["id"] == "export_assessment" && op["kind"] == "export"));

    let mut incoming_counts: HashMap<String, usize> = HashMap::new();
    let mut outgoing_counts: HashMap<String, usize> = HashMap::new();
    for edge in edges {
        if let Some(from) = edge["from"].as_str() {
            *outgoing_counts.entry(from.to_string()).or_insert(0) += 1;
        }
        if let Some(to) = edge["to"].as_str() {
            *incoming_counts.entry(to.to_string()).or_insert(0) += 1;
        }
    }

    let has_branch_node = outgoing_counts.values().any(|&count| count > 1);
    let entry_branches = operations
        .iter()
        .filter(|op| {
            op["id"]
                .as_str()
                .map(|id| incoming_counts.get(id).copied().unwrap_or(0) == 0)
                .unwrap_or(false)
        })
        .count();
    assert!(
        has_branch_node || entry_branches >= 2,
        "workflow should show branching via multi-outgoing node or multiple entry branches"
    );
    assert!(
        incoming_counts
            .get("compile_assessment_context")
            .copied()
            .unwrap_or(0)
            >= 2,
        "compile_assessment_context should join multiple incoming edges"
    );
}
