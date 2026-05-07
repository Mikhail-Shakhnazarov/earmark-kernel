use std::{fs, path::PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn deposit_outputs_machine_readable_json() {
    let dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("note")
        .arg("--title")
        .arg("Hello")
        .arg("--body")
        .arg("world");
    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["contract_version"], "0.2.0");
    assert_eq!(parsed["data"]["ok"], true);
    assert_eq!(parsed["data"]["class"], "note");
}

#[test]
fn query_outputs_machine_readable_json() {
    let dir = tempdir().unwrap();
    let mut deposit = Command::cargo_bin("earmark-cli").unwrap();
    deposit
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("note")
        .arg("--title")
        .arg("Indexed")
        .arg("--body")
        .arg("hello world")
        .assert()
        .success();

    let mut query = Command::cargo_bin("earmark-cli").unwrap();
    query
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("query")
        .arg("--class")
        .arg("note");
    let output = query.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"].as_array().unwrap().len(), 1);
}

#[test]
fn system_activate_outputs_machine_readable_json() {
    let dir = tempdir().unwrap();
    let manifest = dir.path().join("system.yaml");
    fs::write(
        &manifest,
        r#"system_id: pkm-core
namespace: systems/pkm-core
title: PKM Core
description: Example
classes: []
instructions: []
policies: []
workflows: []
compiled_contexts: []
provider_profiles: []
default_compiled_context: null
default_provider_profile: null
runtime_profile:
  execution_surface: runtime_over_folder
  machine_output_default: json
  work_surface_mode: materialized_manifest
activated_at: null
"#,
    )
    .unwrap();

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("system")
        .arg("register")
        .arg(&manifest)
        .assert()
        .success();

    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("system")
        .arg("activate")
        .arg("pkm-core")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["ok"], true);
    assert_eq!(parsed["data"]["system_id"], "pkm-core");
}

#[test]
fn declare_validate_class_outputs_summary() {
    let dir = tempdir().unwrap();
    let workspace = workspace_root();
    let class_path = workspace.join("docs/declarations/examples/classes/finding.yaml");

    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("declare")
        .arg("validate")
        .arg("--kind")
        .arg("class")
        .arg(class_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["ok"], true);
    assert_eq!(parsed["data"]["kind"], "class");
    assert_eq!(parsed["data"]["summary"]["name"], "finding");
}

#[test]
fn declare_explain_workflow_outputs_operations() {
    let dir = tempdir().unwrap();
    let workspace = workspace_root();
    let pattern_path = workspace.join("docs/declarations/examples/workflows/source_to_finding.yaml");

    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("declare")
        .arg("explain")
        .arg("--kind")
        .arg("workflow")
        .arg(pattern_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["ok"], true);
    assert_eq!(parsed["data"]["kind"], "workflow");
    assert_eq!(parsed["data"]["explanation"]["title"], "Workflow source_to_finding");
    assert_eq!(
        parsed["data"]["explanation"]["operations"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn declare_list_examples_outputs_examples() {
    let workspace = workspace_root();
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(&workspace)
        .arg("--json")
        .arg("declare")
        .arg("list-examples")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["ok"], true);
    let examples = parsed["data"]["examples"].as_array().unwrap();
    assert!(!examples.is_empty());
}

#[test]
fn init_outputs_workspace_paths() {
    let dir = tempdir().unwrap();
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["ok"], true);
    assert!(parsed["data"]["paths"]["canonical_dir"]
        .as_str()
        .unwrap()
        .contains(".earmark"));
}

#[test]
fn doctor_outputs_ok() {
    let dir = tempdir().unwrap();
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("doctor")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["ok"], true);
    assert_eq!(parsed["data"]["index_rebuild_ok"], true);
}

#[test]
fn run_list_outputs_json() {
    let dir = tempdir().unwrap();
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("list")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["ok"], true);
    assert!(parsed["data"]["runs"].is_array());
}

#[test]
fn status_outputs_artifact_counts() {
    let dir = tempdir().unwrap();
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("status")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["ok"], true);
    assert!(parsed["data"]["change_set_count"].is_number());
    assert!(parsed["data"]["run_count"].is_number());
    assert!(parsed["data"]["assignment_count_by_status"].is_object());
}

#[test]
fn artifact_explain_missing_id_fails_cleanly() {
    let dir = tempdir().unwrap();
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("failure")
        .arg("explain")
        .arg("missing_failure_id")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["contract_version"], "0.2.0");
    assert_eq!(parsed["ok"], false);
    assert!(parsed["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));
}

#[test]
fn declare_new_scaffolds_file() {
    let dir = tempdir().unwrap();
    let output_path = dir.path().join("declarations/workflows/sample_flow.yaml");
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(workspace_root())
        .arg("--json")
        .arg("declare")
        .arg("new")
        .arg("--kind")
        .arg("workflow")
        .arg("sample_flow")
        .arg("--path")
        .arg(&output_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["ok"], true);
    assert!(output_path.exists());
}

#[test]
fn system_path_manifest_validates() {
    let dir = tempdir().unwrap();
    let manifest = workspace_root().join("examples/research-synthesis/declarations/systems/system.yaml");
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("declare")
        .arg("validate")
        .arg("--kind")
        .arg("system")
        .arg(manifest)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["data"]["ok"], true);
    assert_eq!(parsed["data"]["summary"]["kind"], "path_system_manifest");
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
