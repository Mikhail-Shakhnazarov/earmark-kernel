use assert_cmd::Command;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn workspace_command() -> Command {
    let ws_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.current_dir(ws_root);
    cmd
}

fn setup_and_init_example() -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();

    // 1. em init
    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("init")
        .assert()
        .success();

    // 2. em orchestration init-example
    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("init-example")
        .assert()
        .success();

    (dir, root)
}

#[test]
fn test_orchestration_lifecycle_verification() {
    let (_dir, root) = setup_and_init_example();

    // 1. Create a native JSON task
    let task_json_path = root.join("my_task.json");
    let payload = serde_json::json!({
        "task_id": "complete_lifecycle_task",
        "title": "Complete lifecycle task",
        "goal": "Verify all CLI command interactions work sequentially",
        "priority": "high",
        "status": "proposed"
    });
    fs::write(&task_json_path, serde_json::to_string(&payload).unwrap()).unwrap();

    // 2. em orchestration ingest-task --source native-json
    let ingest_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("ingest-task")
        .arg("--source")
        .arg("native-json")
        .arg(&task_json_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_ingest: Value = serde_json::from_slice(&ingest_output).unwrap();
    let task_oid = parsed_ingest["data"]["tasks"][0]["object_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 3. em orchestration list
    let list_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("list")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_list: Value = serde_json::from_slice(&list_output).unwrap();
    let tasks_list = parsed_list["data"]["tasks"].as_array().unwrap();
    assert!(tasks_list.iter().any(|t| t["object_id"] == task_oid));

    // 4. em orchestration capture-git
    let capture_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("capture-git")
        .arg("--task-id")
        .arg(&task_oid)
        .arg("--phase")
        .arg("before")
        .arg("--commit")
        .arg("abc123def456")
        .arg("--base")
        .arg("main")
        .arg("--head")
        .arg("feature-test")
        .arg("--include-diff-stat")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_capture: Value = serde_json::from_slice(&capture_output).unwrap();
    assert_eq!(parsed_capture["data"]["kind"], "orchestration_git_snapshot");
    assert_eq!(parsed_capture["data"]["phase"], "before");
    assert_eq!(parsed_capture["data"]["commit"], "abc123def456");

    // 5. em orchestration record-gate
    let gate_log_path = root.join("test_run.log");
    fs::write(
        &gate_log_path,
        "Running unit tests...\nAll 5 tests passed successfully!",
    )
    .unwrap();

    let gate_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("record-gate")
        .arg("--task-id")
        .arg(&task_oid)
        .arg("--command")
        .arg("cargo test")
        .arg("--status")
        .arg("passed")
        .arg("--log")
        .arg(&gate_log_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_gate: Value = serde_json::from_slice(&gate_output).unwrap();
    assert_eq!(parsed_gate["data"]["kind"], "orchestration_gate_result");
    assert_eq!(parsed_gate["data"]["status"], "pass");

    // 6. em orchestration show
    let show_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("show")
        .arg(&task_oid)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_show: Value = serde_json::from_slice(&show_output).unwrap();
    assert_eq!(parsed_show["data"]["kind"], "orchestration_task_details");
    assert_eq!(parsed_show["data"]["task_id"], "complete_lifecycle_task");

    let git_snapshots = parsed_show["data"]["git_snapshots"].as_array().unwrap();
    assert_eq!(git_snapshots.len(), 1);
    assert_eq!(git_snapshots[0]["commit"], "abc123def456");

    let gate_results = parsed_show["data"]["gate_results"].as_array().unwrap();
    assert_eq!(gate_results.len(), 1);
    assert_eq!(gate_results[0]["status"], "pass");

    // 7. em orchestration timeline
    let timeline_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("timeline")
        .arg(&task_oid)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_timeline: Value = serde_json::from_slice(&timeline_output).unwrap();
    assert_eq!(parsed_timeline["data"]["kind"], "orchestration_timeline");
    let events = parsed_timeline["data"]["events"].as_array().unwrap();
    // Should contain: the task itself, the git snapshot, and the gate result
    assert!(events.len() >= 3);
}

#[test]
fn test_gate_status_normalization() {
    let (_dir, root) = setup_and_init_example();

    // Create a task to link gate results to
    let task_json_path = root.join("task.json");
    let payload = serde_json::json!({
        "title": "Gate test task",
        "goal": "Test normalization",
        "priority": "low",
        "status": "proposed"
    });
    fs::write(&task_json_path, serde_json::to_string(&payload).unwrap()).unwrap();

    let ingest_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("ingest-task")
        .arg("--source")
        .arg("native-json")
        .arg(&task_json_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_ingest: Value = serde_json::from_slice(&ingest_output).unwrap();
    let task_oid = parsed_ingest["data"]["tasks"][0]["object_id"]
        .as_str()
        .unwrap()
        .to_string();

    let statuses_to_test = vec![
        ("passed", "pass"),
        ("success", "pass"),
        ("ok", "pass"),
        ("failed", "fail"),
        ("error", "fail"),
        ("skip", "skipped"),
    ];

    for (input_status, expected_normalized) in statuses_to_test {
        let gate_output = workspace_command()
            .arg("--root")
            .arg(&root)
            .arg("--json")
            .arg("orchestration")
            .arg("record-gate")
            .arg("--task-id")
            .arg(&task_oid)
            .arg("--command")
            .arg("dummy cmd")
            .arg("--status")
            .arg(input_status)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let parsed_gate: Value = serde_json::from_slice(&gate_output).unwrap();
        assert_eq!(parsed_gate["data"]["status"], expected_normalized);
    }

    // Invalid status should fail
    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("record-gate")
        .arg("--task-id")
        .arg(&task_oid)
        .arg("--command")
        .arg("dummy cmd")
        .arg("--status")
        .arg("invalid_status_name")
        .assert()
        .failure();
}

#[test]
fn test_mock_engram_adapter() {
    let (_dir, root) = setup_and_init_example();

    // 1. Create a mock engram executable (bash script)
    let mock_engram_path = root.join("mock-engram.sh");
    let script_content = r#"#!/usr/bin/env bash
task_id="$3"
if [ "$task_id" = "missing_title" ]; then
  echo "  ID: missing_title"
  echo "  Status: Todo"
elif [ "$task_id" = "not_found" ]; then
  echo "Not found" >&2
  exit 1
elif [ "$task_id" = "controlled_fail" ]; then
  echo "Some unknown engram error" >&2
  exit 1
else
  echo "  ID: $task_id"
  echo "  Title: Verify Earmark Engram Integration"
  echo "  Description: Ensure native-json and engram align perfectly."
  echo "  Priority: High"
  echo "  Status: Todo"
fi
"#;
    fs::write(&mock_engram_path, script_content).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&mock_engram_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&mock_engram_path, perms).unwrap();
    }

    // A. Verify successful ingestion
    let ingest_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .env("ENGRAM_BIN", &mock_engram_path)
        .arg("orchestration")
        .arg("ingest-task")
        .arg("--source")
        .arg("engram")
        .arg("task_999")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_ingest: Value = serde_json::from_slice(&ingest_output).unwrap();
    assert_eq!(parsed_ingest["data"]["source"], "engram");
    let tasks = parsed_ingest["data"]["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "Verify Earmark Engram Integration");
    assert_eq!(tasks[0]["status"], "proposed");

    // B. Verify missing title error handling
    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .env("ENGRAM_BIN", &mock_engram_path)
        .arg("orchestration")
        .arg("ingest-task")
        .arg("--source")
        .arg("engram")
        .arg("missing_title")
        .assert()
        .failure();

    // C. Verify not_found handling
    let not_found_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .env("ENGRAM_BIN", &mock_engram_path)
        .arg("orchestration")
        .arg("ingest-task")
        .arg("--source")
        .arg("engram")
        .arg("not_found")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let parsed_err: Value = serde_json::from_slice(&not_found_output).unwrap();
    assert_eq!(parsed_err["error"]["code"], "not_found");

    // D. Verify controlled error code on missing path/executable
    let missing_bin_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .env("ENGRAM_BIN", "/nonexistent/path/to/engram_bin")
        .arg("orchestration")
        .arg("ingest-task")
        .arg("--source")
        .arg("engram")
        .arg("task_999")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let parsed_missing: Value = serde_json::from_slice(&missing_bin_output).unwrap();
    assert_eq!(parsed_missing["error"]["code"], "argument");
    assert!(parsed_missing["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Engram executable not found"));
}

#[test]
fn test_workflow_declaration_class_validator() {
    let ws_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    let system_yaml_path = ws_root
        .join("examples")
        .join("earmark-dev-orchestration")
        .join("declarations")
        .join("system.yaml");

    assert!(
        system_yaml_path.exists(),
        "system.yaml does not exist at expected path"
    );

    let system_content = fs::read_to_string(&system_yaml_path).unwrap();
    let system_val: Value = serde_yaml::from_str(&system_content).unwrap();

    let mut registered_contracts = HashSet::new();

    // 1. Read and parse all registered classes
    if let Some(classes_list) = system_val["classes"].as_array() {
        for rel_path_val in classes_list {
            let rel_path = rel_path_val.as_str().expect("class path is not a string");
            let abs_path = system_yaml_path.parent().unwrap().join(rel_path);
            assert!(abs_path.exists(), "class file {} does not exist", rel_path);

            let class_content = fs::read_to_string(&abs_path).unwrap();
            let class_val: Value = serde_yaml::from_str(&class_content).unwrap();
            let class_name = class_val["name"]
                .as_str()
                .expect("class does not have a valid name string")
                .to_string();

            registered_contracts.insert(class_name);
        }
    }

    // 2. Read and parse all registered compiled_contexts (which are also valid input/output contracts)
    if let Some(contexts_list) = system_val["compiled_contexts"].as_array() {
        for rel_path_val in contexts_list {
            let rel_path = rel_path_val.as_str().expect("context path is not a string");
            let abs_path = system_yaml_path.parent().unwrap().join(rel_path);
            assert!(
                abs_path.exists(),
                "context file {} does not exist",
                rel_path
            );

            let ctx_content = fs::read_to_string(&abs_path).unwrap();
            let ctx_val: Value = serde_yaml::from_str(&ctx_content).unwrap();
            let ctx_name = ctx_val["name"]
                .as_str()
                .expect("context does not have a valid name string")
                .to_string();

            registered_contracts.insert(ctx_name);
        }
    }

    // 3. Read and validate all workflows
    let workflows_list = system_val["workflows"]
        .as_array()
        .expect("workflows is not an array");
    for rel_path_val in workflows_list {
        let rel_path = rel_path_val
            .as_str()
            .expect("workflow path is not a string");
        let abs_path = system_yaml_path.parent().unwrap().join(rel_path);
        assert!(
            abs_path.exists(),
            "workflow file {} does not exist",
            rel_path
        );

        let wf_content = fs::read_to_string(&abs_path).unwrap();
        let wf_val: Value = serde_yaml::from_str(&wf_content).unwrap();

        let operations = wf_val["operations"]
            .as_array()
            .expect("operations is not an array");
        for op in operations {
            if let Some(inputs) = op["input_contracts"].as_array() {
                for input in inputs {
                    let contract_name = input.as_str().expect("input_contract is not a string");
                    assert!(
                        registered_contracts.contains(contract_name),
                        "Operation {} in workflow {} references input_contract {} which is not declared in system.yaml",
                        op["id"].as_str().unwrap_or("unknown"),
                        rel_path,
                        contract_name
                    );
                }
            }

            if let Some(outputs) = op["output_contracts"].as_array() {
                for output in outputs {
                    let contract_name = output.as_str().expect("output_contract is not a string");
                    assert!(
                        registered_contracts.contains(contract_name),
                        "Operation {} in workflow {} references output_contract {} which is not declared in system.yaml",
                        op["id"].as_str().unwrap_or("unknown"),
                        rel_path,
                        contract_name
                    );
                }
            }
        }
    }
}
