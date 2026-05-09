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
    assert_eq!(parsed["data"]["ok"], true);
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
    assert_eq!(parsed["data"]["ok"], true);
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
    assert!(dir.path().join(".earmark").exists());
    assert!(dir.path().join(".earmark/canonical").exists());
    assert!(dir.path().join(".earmark/derived").exists());
    assert!(dir.path().join(".earmark/work_surfaces").exists());
    assert!(dir.path().join("corpus").exists());
    assert!(dir.path().join(".git").exists());
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
    assert_eq!(parsed["data"]["ok"], true);
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
    assert_eq!(parsed["data"]["ok"], true);
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
    assert_eq!(parsed["data"]["ok"], true);
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
    assert_eq!(parsed["data"]["ok"], true);
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
fn env_root_overrides_config_root() {
    let dir = tempdir().unwrap();
    let cfg_root = dir.path().join("cfg-root");
    let env_root = dir.path().join("env-root");
    fs::create_dir_all(&cfg_root).unwrap();
    fs::create_dir_all(&env_root).unwrap();
    let cfg_path = dir.path().join("em.toml");
    fs::write(&cfg_path, format!("root = \"{}\"\n", cfg_root.display())).unwrap();

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
    assert_eq!(parsed["data"]["root"], env_root.display().to_string());
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
    assert_eq!(parsed["data"]["root"], cli_root.display().to_string());
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
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
    assert_eq!(parsed["data"]["ok"], true);
    assert_eq!(parsed["data"]["kind"], "provider_capabilities");
    assert!(parsed["data"]["providers"].is_array());
}
