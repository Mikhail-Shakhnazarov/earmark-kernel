use std::{collections::BTreeMap, fs, path::PathBuf};

use assert_cmd::Command;
use earmark_core::{
    to_yaml, Kind, Provenance, RuntimeProfile, Standing, SystemDefinition, TransformationFailure,
    VersionRef,
};
use earmark_exec::{ExecutionEngine, ProviderRegistry, WorkflowRunRequest};
use earmark_index::{DerivedIndex, IndexDirtyMarker};
use earmark_store::{GitCanonicalStore, ObjectStore, StoreScanner, StoredObject, StoredPayload};
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
    assert_eq!(parsed["ok"], true);
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
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["system_id"], "pkm-core");
}

#[test]
fn declare_validate_class_outputs_summary() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();
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
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "class");
    assert_eq!(parsed["data"]["summary"]["name"], "finding");
}

#[test]
fn declare_explain_workflow_outputs_operations() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();
    let workspace = workspace_root();
    let pattern_path =
        workspace.join("docs/declarations/examples/workflows/source_to_finding.yaml");

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
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "workflow");
    assert_eq!(
        parsed["data"]["explanation"]["title"],
        "Workflow source_to_finding"
    );
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
    let dir = tempdir().unwrap();
    let examples_dir = dir.path().join("docs/declarations/examples/classes");
    fs::create_dir_all(&examples_dir).unwrap();
    fs::write(
        examples_dir.join("finding.yaml"),
        "name: finding\nversion: 1\n",
    )
    .unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("declare")
        .arg("list-examples")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["examples"].as_array().unwrap().len(), 1);
    assert!(parsed["data"]["summary"]
        .as_str()
        .unwrap()
        .contains("1 declaration examples found"));
}

#[test]
fn declare_list_examples_empty_in_fresh_workspace() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("declare")
        .arg("list-examples")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["examples"].as_array().unwrap().len(), 0);
    assert!(parsed["data"]["summary"]
        .as_str()
        .unwrap()
        .contains("No workspace-local declaration examples found"));
    assert_eq!(parsed["data"]["next_commands"].as_array().unwrap().len(), 0);
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
    assert_eq!(parsed["ok"], true);
    assert!(parsed["data"]["paths"]["canonical_dir"]
        .as_str()
        .unwrap()
        .contains(".earmark"));
    assert!(dir.path().join(".earmark").exists());
    assert!(dir.path().join(".earmark/canonical").exists());
    assert!(dir.path().join(".earmark/derived").exists());
    assert!(dir.path().join(".earmark/work_surfaces").exists());
    assert!(dir.path().join("corpus").exists());
    assert!(dir.path().join(".earmark/canonical/.git").exists());
}

#[test]
fn doctor_reports_uninitialized_workspace_without_side_effects() {
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
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["ok"], false);
    assert_eq!(parsed["data"]["summary"], "workspace is not initialized");
    assert!(!dir.path().join(".earmark").exists());
    assert!(!dir.path().join(".git").exists());
}

#[test]
fn run_list_outputs_json() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();
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
    assert_eq!(parsed["ok"], true);
    assert!(parsed["data"]["runs"].is_array());
}

#[test]
fn status_outputs_artifact_counts() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();
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
    assert_eq!(parsed["ok"], true);
    assert!(parsed["data"]["change_set_count"].is_number());
    assert!(parsed["data"]["run_count"].is_number());
    assert!(parsed["data"]["assignment_count_by_status"].is_object());
}

#[test]
fn artifact_explain_missing_id_fails_cleanly() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();
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
    assert_eq!(parsed["ok"], true);
    assert!(output_path.exists());
}

#[test]
fn system_path_manifest_validates() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();
    let manifest =
        workspace_root().join("examples/research-synthesis/declarations/systems/system.yaml");
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
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["summary"]["kind"], "path_system_manifest");
}

#[test]
fn completions_command_generates_bash_script() {
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("completions")
        .arg("bash")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let rendered = String::from_utf8(output).unwrap();
    assert!(rendered.contains("_em()"));
}

#[test]
fn status_requires_initialized_workspace_without_creating_layout() {
    let dir = tempdir().unwrap();

    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("status")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], false);
    assert!(parsed["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not initialized"));
    assert!(!dir.path().join(".earmark").exists());
    assert!(!dir.path().join(".git").exists());
}

#[test]
fn query_requires_initialized_workspace_without_creating_layout() {
    let dir = tempdir().unwrap();

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("query")
        .arg("--class")
        .arg("note")
        .assert()
        .failure();

    assert!(!dir.path().join(".earmark").exists());
    assert!(!dir.path().join(".git").exists());
}

#[test]
fn completions_does_not_touch_workspace() {
    let dir = tempdir().unwrap();

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("completions")
        .arg("bash")
        .assert()
        .success();

    assert!(!dir.path().join(".earmark").exists());
    assert!(!dir.path().join(".git").exists());
}

#[test]
fn workflow_run_uses_config_default_system_id() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".earmark");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        "default_system_id = \"sys_research_synthesis\"\njson = true\n",
    )
    .unwrap();

    let workspace = workspace_root();
    let manifest = workspace.join("examples/research-synthesis/declarations/systems/system.yaml");
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("system")
        .arg("register")
        .arg(&manifest)
        .assert()
        .success();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("system")
        .arg("activate")
        .arg("sys_research_synthesis")
        .assert()
        .success();

    let deposit_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("source_note")
        .arg("--title")
        .arg("Test")
        .arg("--body")
        .arg("Body")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&deposit_output).unwrap();
    let object_id = parsed["data"]["object_id"].as_str().unwrap();

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("workflow")
        .arg("run")
        .arg("research_synthesis")
        .arg("--with")
        .arg(object_id)
        .assert()
        .success();
}

#[test]
fn test_deposit_rejection_in_active_system() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".earmark");
    fs::create_dir_all(&config_dir).unwrap();

    // 1. Setup classes and system (using research-synthesis example)
    let workspace = workspace_root();
    let system_manifest =
        workspace.join("examples/research-synthesis/declarations/systems/system.yaml");

    // Register and activate system
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("system")
        .arg("register")
        .arg(&system_manifest)
        .assert()
        .success();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("system")
        .arg("activate")
        .arg("sys_research_synthesis")
        .assert()
        .success();

    // 2. Setup env for system context
    unsafe { std::env::set_var("EM_SYSTEM_ID", "sys_research_synthesis") };

    let canonical_path = dir.path().join(".earmark").join("canonical");
    let count_before = fs::read_dir(&canonical_path).unwrap().count();

    // 3. Deposit non-admitted class should fail
    // research-synthesis admits 'source_note', 'finding', etc. but NOT 'unauthorized_class'
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("deposit")
        .arg("--class")
        .arg("unauthorized_class")
        .arg("--body")
        .arg("some body")
        .assert()
        .failure();

    let count_after = fs::read_dir(&canonical_path).unwrap().count();
    assert_eq!(
        count_before, count_after,
        "No new objects should be created on admission rejection"
    );

    unsafe { std::env::remove_var("EM_SYSTEM_ID") };
}

#[test]
fn test_deposit_fails_on_bad_system_context() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("init")
        .assert()
        .success();

    // Set a system ID that doesn't exist
    unsafe { std::env::set_var("EM_SYSTEM_ID", "non_existent_system") };

    let canonical_path = dir.path().join(".earmark").join("canonical");
    let count_before = fs::read_dir(&canonical_path).unwrap().count();

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("deposit")
        .arg("--class")
        .arg("any")
        .arg("--body")
        .arg("body")
        .assert()
        .failure();

    let count_after = fs::read_dir(&canonical_path).unwrap().count();
    assert_eq!(
        count_before, count_after,
        "No new objects should be created on bad system context failure"
    );

    unsafe { std::env::remove_var("EM_SYSTEM_ID") };
}

#[test]
fn env_root_overrides_config_root() {
    let dir = tempdir().unwrap();
    let cfg_root = dir.path().join("cfg-root");
    let env_root = dir.path().join("env-root");
    fs::create_dir_all(&cfg_root).unwrap();
    fs::create_dir_all(&env_root).unwrap();
    let cfg_path = dir.path().join("em.toml");
    fs::write(
        &cfg_path,
        format!(
            "root = \"{}\"\n",
            cfg_root.to_string_lossy().replace("\\", "\\\\")
        ),
    )
    .unwrap();

    unsafe { std::env::set_var("EM_ROOT", env_root.as_os_str()) };
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--config")
        .arg(&cfg_path)
        .arg("--json")
        .arg("init")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    unsafe { std::env::remove_var("EM_ROOT") };

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    let actual_root = parsed["data"]["root"].as_str().unwrap();
    assert_paths_eq(actual_root, &env_root);
}

#[test]
fn cli_root_overrides_env_root() {
    let dir = tempdir().unwrap();
    let env_root = dir.path().join("env-root");
    let cli_root = dir.path().join("cli-root");
    fs::create_dir_all(&env_root).unwrap();
    fs::create_dir_all(&cli_root).unwrap();

    unsafe { std::env::set_var("EM_ROOT", env_root.as_os_str()) };
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(&cli_root)
        .arg("--json")
        .arg("init")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    unsafe { std::env::remove_var("EM_ROOT") };

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    let actual_root = parsed["data"]["root"].as_str().unwrap();
    assert_paths_eq(actual_root, &cli_root);
}

#[test]
fn doctor_reports_healthy_workspace_after_deposit() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("note")
        .arg("--title")
        .arg("Test")
        .arg("--body")
        .arg("content")
        .assert()
        .success();

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
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["store_scan_ok"], true);
    assert_eq!(parsed["data"]["index_exists"], true);
    assert_eq!(parsed["data"]["index_open_ok"], true);
    assert_eq!(parsed["data"]["counts_match"], true);
    assert!(parsed["data"]["canonical_object_count"].as_u64().unwrap() > 0);
    assert!(parsed["data"]["indexed_object_count"].as_u64().unwrap() > 0);
    assert_eq!(
        parsed["data"]["canonical_object_count"],
        parsed["data"]["indexed_object_count"]
    );
}

#[test]
fn doctor_reports_index_missing_after_deletion() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    let index_path = dir
        .path()
        .join(".earmark")
        .join("derived")
        .join("index.sqlite");
    let _ = std::fs::remove_file(&index_path);

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
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["ok"], false);
    assert_eq!(parsed["data"]["store_scan_ok"], true);
    assert_eq!(parsed["data"]["index_exists"], false);
    assert_eq!(parsed["data"]["index_open_ok"], false);
    assert_eq!(parsed["data"]["counts_match"], false);
    assert_eq!(
        parsed["data"]["canonical_object_count"].as_u64().unwrap(),
        0
    );
}

#[test]
fn doctor_reports_dirty_index_without_implicit_repair() {
    let dir = tempdir().unwrap();
    let root = dir.path();

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
        .arg("deposit")
        .arg("--class")
        .arg("note")
        .arg("--title")
        .arg("dirty")
        .arg("--body")
        .arg("index marker")
        .assert()
        .success();

    let index = DerivedIndex::open(root).unwrap();
    index
        .mark_dirty(IndexDirtyMarker {
            schema_version: "v1".to_string(),
            reason: "test_marker".to_string(),
            operation: "test".to_string(),
            timestamp: chrono::Utc::now(),
            object_ids: vec![],
            version_ids: vec![],
        })
        .unwrap();

    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("doctor")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["ok"], false);
    assert_eq!(parsed["data"]["index_is_dirty"], true);

    assert!(index.dirty_status().unwrap().is_some());
}

#[test]
fn doctor_repair_index_rebuilds_and_clears_dirty_marker() {
    let dir = tempdir().unwrap();
    let root = dir.path();

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
        .arg("deposit")
        .arg("--class")
        .arg("note")
        .arg("--title")
        .arg("repair")
        .arg("--body")
        .arg("dirty index")
        .assert()
        .success();

    let index = DerivedIndex::open(root).unwrap();
    index
        .mark_dirty(IndexDirtyMarker {
            schema_version: "v1".to_string(),
            reason: "test_repair".to_string(),
            timestamp: chrono::Utc::now(),
            operation: "test".to_string(),
            object_ids: vec![],
            version_ids: vec![],
        })
        .unwrap();
    assert!(index.dirty_status().unwrap().is_some());

    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("doctor")
        .arg("--repair-index")
        .assert()
        .success();

    let doctor_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("doctor")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&doctor_output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["index_is_dirty"], false);
    assert_eq!(parsed["data"]["counts_match"], true);
}

#[test]
fn demo_path_research_synthesis_full_workflow() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let repo_root = workspace_root();

    let system_manifest =
        repo_root.join("examples/research-synthesis/declarations/systems/system.yaml");
    let seed_note_1 =
        repo_root.join("examples/research-synthesis/data/seed_notes/note_1_benefits.md");
    let seed_note_2 =
        repo_root.join("examples/research-synthesis/data/seed_notes/note_2_challenges.md");

    // 1. init
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("init")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);

    // 2. system register
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("system")
        .arg("register")
        .arg(&system_manifest)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "system_registration");

    // 3. system activate
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("system")
        .arg("activate")
        .arg("sys_research_synthesis")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["system_id"], "sys_research_synthesis");

    // 4. deposit seed note 1
    let output = Command::cargo_bin("earmark-cli")
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
        .arg(&seed_note_1)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["class"], "source_note");
    let note1_id = parsed["data"]["object_id"].as_str().unwrap().to_string();

    // 5. deposit seed note 2
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("source_note")
        .arg("--title")
        .arg("The Cost of Heterogeneity")
        .arg("--payload-file")
        .arg(&seed_note_2)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["class"], "source_note");

    // 6. query source notes
    let output = Command::cargo_bin("earmark-cli")
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
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    let results = parsed["data"].as_array().unwrap();
    assert_eq!(results.len(), 2);

    // 7. workflow run
    let output = Command::cargo_bin("earmark-cli")
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
        .arg(&note1_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["status"], "completed");
    let run_id = parsed["data"]["run_id"].as_str().unwrap().to_string();
    assert!(!parsed["data"]["created_assignments"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!parsed["data"]["created_change_sets"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!parsed["data"]["created_handoffs"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(parsed["data"]["output_count"].as_u64().unwrap() > 0);

    // 8. run explain
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("run")
        .arg("explain")
        .arg("latest")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "run");

    // 9. run timeline
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("run")
        .arg("timeline")
        .arg("latest")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);

    // 10. query findings
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("query")
        .arg("--class")
        .arg("finding")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    let findings = parsed["data"].as_array().unwrap();
    assert!(!findings.is_empty());
    let finding_id = findings[0]["object_id"].as_str().unwrap().to_string();

    // 11. query summaries
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("query")
        .arg("--class")
        .arg("summary")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    let summaries = parsed["data"].as_array().unwrap();
    assert!(!summaries.is_empty());
    let summary_id = summaries[0]["object_id"].as_str().unwrap().to_string();
    assert_ne!(finding_id, summary_id);

    // 12. handoff explain (use first handoff from the run)
    let output = Command::cargo_bin("earmark-cli")
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
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    let handoffs = parsed["data"]["artifact"]["handoffs"]
        .as_array()
        .expect("run artifacts should contain a non-empty handoffs array");
    assert!(
        !handoffs.is_empty(),
        "demo workflow should create at least one handoff"
    );
    let first_handoff = handoffs[0]
        .as_str()
        .expect("handoff entries should be handoff IDs");
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("handoff")
        .arg("explain")
        .arg(first_handoff)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert!(
        parsed["data"]["kind"]
            .as_str()
            .is_some_and(|k| k.contains("handoff")),
        "handoff explain response should identify a handoff, got {:?}",
        parsed["data"]["kind"]
    );
    assert!(
        parsed["data"]["summary"]
            .as_str()
            .is_some_and(|s| s.contains(first_handoff)),
        "handoff explain summary should reference the handoff id"
    );

    // 13. run graph
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("run")
        .arg("graph")
        .arg("latest")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);

    // 14. report generation
    let report_path = root.join("research_report.html");
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("report")
        .arg("run")
        .arg("latest")
        .arg("--output")
        .arg(&report_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "report_generation");
    assert!(report_path.exists());
    let report_bytes = std::fs::read(&report_path).unwrap();
    assert!(!report_bytes.is_empty());

    // 15. doctor reports healthy
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("doctor")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["store_scan_ok"], true);
    assert_eq!(parsed["data"]["index_exists"], true);
    assert!(parsed["data"]["canonical_object_count"].as_u64().unwrap() > 0);
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn assert_paths_eq<P1: AsRef<std::path::Path>, P2: AsRef<std::path::Path>>(
    actual: P1,
    expected: P2,
) {
    let a = actual
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| actual.as_ref().to_path_buf());
    let e = expected
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| expected.as_ref().to_path_buf());
    assert_eq!(
        a.to_string_lossy().to_lowercase(),
        e.to_string_lossy().to_lowercase()
    );
}

#[test]
fn provider_capabilities_outputs_json() {
    let dir = tempdir().unwrap();
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("provider")
        .arg("capabilities")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "provider_capabilities");
    assert!(parsed["data"]["providers"].is_array());
}

#[test]
fn failure_cli_inspection_commands_with_real_failure() {
    let dir = tempdir().unwrap();

    // Init workspace via CLI
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    // Create objects and run a failing workflow via Rust API
    let (failure_id, run_id) = {
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let note = StoredObject::new(
            Kind::Object,
            Some("note".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown("seed body"),
            vec![],
        );
        store.write_object(&note).unwrap();

        let system = SystemDefinition {
            system_id: "test_system".to_string(),
            namespace: "systems/test".to_string(),
            title: "Test System".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![],
            runtime_profile: RuntimeProfile {
                execution_surface: "runtime_over_folder".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "materialized_manifest".to_string(),
            },
            activated_at: None,
        };
        let system_obj = StoredObject::new(
            Kind::SystemDefinition,
            Some("system_definition".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&system).unwrap()),
            vec![],
        );
        let system_ref = store.write_object(&system_obj).unwrap();

        let workflow_yaml = r#"name: fail_flow
version: "1"
description: fail during transform
operations:
  - id: op_fail
    kind: transform
    input_contracts: [note]
    output_contracts: [note]
    instruction: null
    compiled_context: null
    policy: null
    provider_profile: null
edges: []
guards: []
"#;
        let workflow_obj = StoredObject::new(
            Kind::Workflow,
            Some("composition_workflow".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_yaml(workflow_yaml),
            vec![],
        );
        let workflow_ref = store.write_object(&workflow_obj).unwrap();
        index.rebuild_from_store(&store).unwrap();

        let registry = ProviderRegistry::default();
        let engine = ExecutionEngine {
            store: &store,
            index: &index,
            provider_service: &registry,
        };

        let result = engine.run_workflow(WorkflowRunRequest {
            run_id: "cli_test_fail_run".to_string(),
            system_definition: VersionRef::new(system_ref.id, system_ref.version_id),
            workflow: VersionRef::new(workflow_ref.id, workflow_ref.version_id),
            inputs: vec![note.object_ref()],
            handoff_manifest: None,
            transition_assignment: None,
            operator_approved: true,
        });
        assert!(result.is_err());

        let objects = store.scan_objects().unwrap();
        let failure_obj = objects
            .iter()
            .find(|obj| obj.envelope.kind == Kind::TransformationFailure)
            .expect("TransformationFailure not found");
        let failure: TransformationFailure =
            serde_json::from_slice(&failure_obj.payload.bytes).unwrap();

        (
            failure_obj.envelope.id.as_str().to_string(),
            failure.run_id.clone(),
        )
    };

    // Test failure explain exposes related context and next commands
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("failure")
        .arg("explain")
        .arg(&failure_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["kind"], "failure");
    assert_eq!(parsed["data"]["related"]["run_id"], run_id);
    assert!(parsed["data"]["next_commands"].as_array().unwrap().len() >= 2);
    assert!(parsed["data"]["artifact"]["input_object_ids"].is_array());

    // Test failure list --run-id returns the failure
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("failure")
        .arg("list")
        .arg("--run-id")
        .arg(&run_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    let failures = parsed["data"]["failures"].as_array().unwrap();
    assert!(failures
        .iter()
        .any(|f| f["failure_id"].as_str() == Some(&failure_id)));
    assert!(failures
        .iter()
        .any(|f| f["assignment_id"].as_str().is_some()));

    // Test run artifacts includes the failure
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("artifacts")
        .arg(&run_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["ok"], true);
    let artifact_failures = parsed["data"]["artifact"]["failures"].as_array().unwrap();
    assert!(artifact_failures
        .iter()
        .any(|f| f.as_str() == Some(&failure_id)));
}

#[test]
fn json_output_uses_versioned_envelope() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    for cmd_and_args in [
        vec!["status"],
        vec!["run", "list"],
        vec!["failure", "list"],
        vec!["doctor"],
        vec!["provider", "capabilities"],
    ] {
        let output = Command::cargo_bin("earmark-cli")
            .unwrap()
            .arg("--root")
            .arg(dir.path())
            .arg("--json")
            .args(&cmd_and_args)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let parsed: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(
            parsed["contract_version"],
            "0.2.0",
            "command '{}' missing contract_version",
            cmd_and_args.join(" ")
        );
        assert!(
            parsed["data"].is_object(),
            "command '{}' missing data envelope",
            cmd_and_args.join(" ")
        );
    }
}

#[test]
fn missing_run_id_fails_cleanly_in_json_mode() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("show")
        .arg("nonexistent_run")
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
fn missing_assignment_id_fails_cleanly_in_json_mode() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("assignment")
        .arg("show")
        .arg("nonexistent_assignment")
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
fn missing_relation_id_fails_cleanly_in_json_mode() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("relation")
        .arg("show")
        .arg("nonexistent_relation")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed["contract_version"], "0.2.0");
    assert_eq!(parsed["ok"], false);
    let msg = parsed["error"]["message"].as_str().unwrap();
    assert!(
        msg.contains("invalid") || msg.contains("not found"),
        "expected error about invalid/not found relation, got: {}",
        msg
    );
}

#[test]
fn latest_run_resolves_correctly() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    // Create a run via Rust API
    use earmark_core::{
        to_yaml, Provenance, RuntimeProfile, Standing, SystemDefinition, VersionRef,
    };
    use earmark_exec::{ExecutionEngine, ProviderRegistry, WorkflowRunRequest};
    use earmark_index::DerivedIndex;
    use earmark_store::{GitCanonicalStore, ObjectStore, StoredObject, StoredPayload};
    use std::collections::BTreeMap;

    let store = GitCanonicalStore::new(dir.path());
    let index = DerivedIndex::open(dir.path()).unwrap();

    let system = SystemDefinition {
        system_id: "test_system".to_string(),
        namespace: "systems/test".to_string(),
        title: "Test System".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![],
        runtime_profile: RuntimeProfile {
            execution_surface: "runtime_over_folder".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "materialized_manifest".to_string(),
        },
        activated_at: None,
    };
    let system_obj = StoredObject::new(
        earmark_core::Kind::SystemDefinition,
        Some("system_definition".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_yaml(to_yaml(&system).unwrap()),
        vec![],
    );
    let system_ref = store.write_object(&system_obj).unwrap();

    let workflow_yaml = r#"name: simple_flow
version: "1"
description: a simple flow
operations:
  - id: op_one
    kind: nop
    input_contracts: [note]
    output_contracts: [note]
edges: []
guards: []
"#;
    let workflow_obj = StoredObject::new(
        earmark_core::Kind::Workflow,
        Some("composition_workflow".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_yaml(workflow_yaml),
        vec![],
    );
    let workflow_ref = store.write_object(&workflow_obj).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let note = StoredObject::new(
        earmark_core::Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("seed body"),
        vec![],
    );
    store.write_object(&note).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let registry = ProviderRegistry::default();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let sys_def = VersionRef::new(system_ref.id.clone(), system_ref.version_id.clone());
    let wf_def = VersionRef::new(workflow_ref.id.clone(), workflow_ref.version_id.clone());

    // Run records are persisted even on failure
    let _ = engine.run_workflow(WorkflowRunRequest {
        run_id: "run_first".to_string(),
        system_definition: sys_def.clone(),
        workflow: wf_def.clone(),
        inputs: vec![note.object_ref()],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    });

    let note2 = StoredObject::new(
        earmark_core::Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("seed body 2"),
        vec![],
    );
    store.write_object(&note2).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let _ = engine.run_workflow(WorkflowRunRequest {
        run_id: "run_second".to_string(),
        system_definition: sys_def,
        workflow: wf_def,
        inputs: vec![note2.object_ref()],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    });

    // Now test that 'latest' resolves to the most recent run
    let output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("explain")
        .arg("latest")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).unwrap();
    let id = parsed["data"]["id"].as_str().unwrap();
    assert_eq!(
        id, "run_second",
        "latest should resolve to the most recent run"
    );
}

#[test]
fn run_show_and_explain_are_distinct() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    // Create a run via Rust API
    use earmark_core::{
        to_yaml, Provenance, RuntimeProfile, Standing, SystemDefinition, VersionRef,
    };
    use earmark_exec::{ExecutionEngine, ProviderRegistry, WorkflowRunRequest};
    use earmark_index::DerivedIndex;
    use earmark_store::{GitCanonicalStore, ObjectStore, StoredObject, StoredPayload};
    use std::collections::BTreeMap;

    let store = GitCanonicalStore::new(dir.path());
    let index = DerivedIndex::open(dir.path()).unwrap();

    let system = SystemDefinition {
        system_id: "test_system".to_string(),
        namespace: "systems/test".to_string(),
        title: "Test System".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![],
        runtime_profile: RuntimeProfile {
            execution_surface: "runtime_over_folder".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "materialized_manifest".to_string(),
        },
        activated_at: None,
    };
    let system_obj = StoredObject::new(
        earmark_core::Kind::SystemDefinition,
        Some("system_definition".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_yaml(to_yaml(&system).unwrap()),
        vec![],
    );
    let system_ref = store.write_object(&system_obj).unwrap();

    let workflow_yaml = r#"name: fail_flow
version: "1"
description: fail during transform
operations:
  - id: op_fail
    kind: transform
    input_contracts: [note]
    output_contracts: [note]
    instruction: null
    compiled_context: null
    policy: null
    provider_profile: null
edges: []
guards: []
"#;
    let workflow_obj = StoredObject::new(
        earmark_core::Kind::Workflow,
        Some("composition_workflow".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_yaml(workflow_yaml),
        vec![],
    );
    let workflow_ref = store.write_object(&workflow_obj).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let note = StoredObject::new(
        earmark_core::Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("seed body"),
        vec![],
    );
    store.write_object(&note).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let registry = ProviderRegistry::default();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let result = engine.run_workflow(WorkflowRunRequest {
        run_id: "run_show_vs_explain".to_string(),
        system_definition: VersionRef::new(system_ref.id, system_ref.version_id),
        workflow: VersionRef::new(workflow_ref.id, workflow_ref.version_id),
        inputs: vec![note.object_ref()],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    });
    // The run record is persisted even on failure
    let _ = result;
    let run_id = "run_show_vs_explain".to_string();

    // run show should be raw (no ok/kind/id/summary structure)
    let show_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("show")
        .arg(&run_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let show_parsed: Value = serde_json::from_slice(&show_output).unwrap();
    // show is raw run record inside data envelope
    assert!(
        show_parsed["data"]["run_id"].is_string(),
        "run show should contain raw run_id at top level"
    );
    assert!(
        show_parsed["data"]["status"].is_string(),
        "run show should contain raw status"
    );
    // show should NOT have related or next_commands at top level
    assert!(
        show_parsed["data"].get("related").is_none(),
        "run show should not contain related context"
    );
    assert!(
        show_parsed["data"].get("next_commands").is_none(),
        "run show should not contain next_commands"
    );

    // run explain should have interpreted structure
    let explain_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("run")
        .arg("explain")
        .arg(&run_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let explain_parsed: Value = serde_json::from_slice(&explain_output).unwrap();
    assert_eq!(explain_parsed["ok"], true);
    assert_eq!(explain_parsed["data"]["kind"], "run");
    assert!(explain_parsed["data"]["related"].is_object());
    assert!(explain_parsed["data"]["next_commands"].is_array());
    assert!(explain_parsed["data"]["artifact"].is_object());
}

#[test]
fn next_commands_contain_valid_syntax() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    // Check next_commands in commands that don't require run data
    for cmd_and_args in [
        vec!["init"],
        vec!["status"],
        vec!["run", "list"],
        vec!["failure", "list"],
        vec!["doctor"],
    ] {
        let output = Command::cargo_bin("earmark-cli")
            .unwrap()
            .arg("--root")
            .arg(dir.path())
            .arg("--json")
            .args(&cmd_and_args)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let parsed: Value = serde_json::from_slice(&output).unwrap();
        let cmds = parsed["data"]["next_commands"].as_array();
        if let Some(cmds) = cmds {
            for cmd in cmds {
                let s = cmd.as_str().unwrap();
                assert!(
                    s.starts_with("em "),
                    "next_command '{}' should start with 'em '",
                    s
                );
                assert!(
                    !s.contains("<") || s.contains(">"),
                    "next_command '{}' should use valid angle brackets if used",
                    s
                );
            }
        }
    }
}
