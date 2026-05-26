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

    // 3. git init (required for capture-git)
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&root)
        .output()
        .unwrap();

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
    assert_eq!(parsed_show["data"]["kind"], "orchestration_work_item_show");
    assert_eq!(parsed_show["data"]["work_item_id"], task_oid);

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
        "task_id": "gate-test-task",
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

#[test]
fn test_explain_dispatch_latest_resolves_newest_dispatch() {
    let (_dir, root) = setup_and_init_example();

    // 1. Create a task
    let task_json_path = root.join("task.json");
    let payload = serde_json::json!({
        "task_id": "latest-test-task",
        "title": "Latest test task",
        "goal": "Verify latest resolution",
        "priority": "low",
        "status": "proposed"
    });
    fs::write(&task_json_path, serde_json::to_string(&payload).unwrap()).unwrap();

    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("orchestration")
        .arg("ingest-task")
        .arg("--source")
        .arg("native-json")
        .arg(&task_json_path)
        .assert()
        .success();

    // 2. Ingest manifest attempt 1
    let manifest1_path = root.join("manifest1.md");
    fs::write(
        &manifest1_path,
        "---\ntask_uuid: latest-test-task\nattempt_number: 1\n---\n## Objective\nAttempt 1",
    )
    .unwrap();

    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("orchestration")
        .arg("ingest-manifest")
        .arg(&manifest1_path)
        .assert()
        .success();

    // Small sleep to ensure different timestamps if the resolution is low
    std::thread::sleep(std::time::Duration::from_millis(100));

    // 3. Ingest manifest attempt 2
    let manifest2_path = root.join("manifest2.md");
    fs::write(
        &manifest2_path,
        "---\ntask_uuid: latest-test-task\nattempt_number: 2\n---\n## Objective\nAttempt 2",
    )
    .unwrap();

    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("orchestration")
        .arg("ingest-manifest")
        .arg(&manifest2_path)
        .assert()
        .success();

    // 4. em orchestration explain-dispatch latest
    let explain_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("explain-dispatch")
        .arg("latest")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&explain_output).unwrap();
    assert_eq!(parsed["data"]["kind"], "orchestration_dispatch_explanation");
    assert_eq!(parsed["data"]["payload"]["attempt"], 2);
}

#[test]
fn test_explain_dispatch_latest_no_dispatches() {
    let (_dir, root) = setup_and_init_example();

    let output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("explain-dispatch")
        .arg("latest")
        .assert()
        .failure()
        .get_output()
        .clone();

    let stdout_msg = String::from_utf8_lossy(&output.stdout);
    let stderr_msg = String::from_utf8_lossy(&output.stderr);
    let all_msg = format!("{}{}", stdout_msg, stderr_msg);
    assert!(all_msg.contains("no dispatch records found"));
}

#[test]
fn test_ingest_manifest_with_bullet_gates() {
    let (_dir, root) = setup_and_init_example();

    // 1. Create a task
    let task_json_path = root.join("task.json");
    let payload = serde_json::json!({
        "task_id": "gates-bullets",
        "title": "Gates bullets task",
        "goal": "Verify bullet gates",
        "priority": "low",
        "status": "proposed"
    });
    fs::write(&task_json_path, serde_json::to_string(&payload).unwrap()).unwrap();

    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("orchestration")
        .arg("ingest-task")
        .arg("--source")
        .arg("native-json")
        .arg(&task_json_path)
        .assert()
        .success();

    // 2. Ingest manifest with bullet gates
    let manifest_path = root.join("manifest_bullets.md");
    fs::write(
        &manifest_path,
        r#"---
task_uuid: gates-bullets
attempt_number: 1
---

## Objective

Verify bullet gates are parsed.

## Local Gates

- cargo fmt --all -- --check
- cargo test --workspace

## Target Files

- README.md
"#,
    )
    .unwrap();

    let ingest_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("ingest-manifest")
        .arg(&manifest_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&ingest_output).unwrap();
    let gates = parsed["data"]["local_gates"].as_array().unwrap();
    assert_eq!(gates.len(), 2);
    assert!(gates[0].as_str().unwrap().contains("cargo fmt"));
    assert!(gates[1].as_str().unwrap().contains("cargo test"));
}

#[test]
fn orchestration_writes_are_immediately_queryable_without_repair_rebuild() {
    let (_dir, root) = setup_and_init_example();

    // 1. Ingest a task
    let task_json_path = root.join("task.json");
    let payload = serde_json::json!({
        "task_id": "incremental-test-task",
        "title": "Incremental test task",
        "goal": "Verify incremental indexing",
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

    // 2. Immediately list/show it - should work WITHOUT rebuild
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
    assert!(parsed_list["data"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["object_id"] == task_oid));

    // 3. Ingest a dispatch
    let manifest_path = root.join("manifest.md");
    fs::write(
        &manifest_path,
        "---\ntask_uuid: incremental-test-task\nattempt_number: 1\n---\n## Objective\nIncrementality test",
    )
    .unwrap();

    let dispatch_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("ingest-manifest")
        .arg(&manifest_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_dispatch: Value = serde_json::from_slice(&dispatch_output).unwrap();
    let dispatch_oid = parsed_dispatch["data"]["object_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 4. Immediately show it
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

    let _parsed_show: Value = serde_json::from_slice(&show_output).unwrap();
    // This currently needs Stage 4 (transitive graph) for dispatch artifact visibility if we show task_oid,
    // but we can check if the dispatch itself is showable.
    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("show")
        .arg(&dispatch_oid)
        .assert()
        .success();

    // 5. Record a gate result
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
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_gate: Value = serde_json::from_slice(&gate_output).unwrap();
    let gate_oid = parsed_gate["data"]["gate_object_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 6. Verify gate visibility
    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("show")
        .arg(&gate_oid)
        .assert()
        .success();
}

#[test]
fn test_record_context() {
    let (_dir, root) = setup_and_init_example();

    // 1. Ingest a task
    let task_json_path = root.join("task.json");
    let payload = serde_json::json!({
        "task_id": "context-task",
        "title": "Context task",
        "goal": "Verify context recording",
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

    // 2. Register JSON context
    let json_context_path = root.join("context.json");
    let json_payload = serde_json::json!({
        "environment": "ci",
        "tools": ["rustc", "cargo"]
    });
    fs::write(
        &json_context_path,
        serde_json::to_string(&json_payload).unwrap(),
    )
    .unwrap();

    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("record-context")
        .arg("--task-id")
        .arg(&task_oid)
        .arg(&json_context_path)
        .assert()
        .success();

    // 3. Register text context
    let text_context_path = root.join("context.txt");
    fs::write(&text_context_path, "Manual context notes").unwrap();

    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("record-context")
        .arg("--task-id")
        .arg(&task_oid)
        .arg(&text_context_path)
        .assert()
        .success();
}

#[test]
fn test_ingest_manifest_with_context_linking() {
    let (_dir, root) = setup_and_init_example();

    // 1. Ingest a task
    let task_json_path = root.join("task.json");
    let payload = serde_json::json!({
        "task_id": "link-ctx-task",
        "title": "Link context task",
        "goal": "Verify context linking",
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

    // 2. Record context
    let context_path = root.join("context.json");
    fs::write(&context_path, "{}").unwrap();

    let context_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("record-context")
        .arg("--task-id")
        .arg(&task_oid)
        .arg(&context_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_context: Value = serde_json::from_slice(&context_output).unwrap();
    let context_oid = parsed_context["data"]["object_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 3. Ingest manifest with context_id
    let manifest_path = root.join("manifest.md");
    fs::write(
        &manifest_path,
        "---\ntask_uuid: link-ctx-task\nattempt_number: 1\n---\n## Objective\nLinking test",
    )
    .unwrap();

    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("ingest-manifest")
        .arg("--context-id")
        .arg(&context_oid)
        .arg(&manifest_path)
        .assert()
        .success();
}

#[test]
fn review_updates_existing_task_head_without_creating_duplicate_work_item() {
    let (_dir, root) = setup_and_init_example();

    // 1. Ingest a work item
    let task_json_path = root.join("task.json");
    let payload = serde_json::json!({
        "task_id": "review-head-update-task",
        "title": "Review Head Update Test",
        "goal": "Verify same-object update",
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
    let task_vid_before = parsed_ingest["data"]["tasks"][0]["version_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. Perform review
    let review_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("review")
        .arg(&task_oid)
        .arg("--decision")
        .arg("accepted")
        .arg("--comment")
        .arg("LGTM")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_review: Value = serde_json::from_slice(&review_output).unwrap();
    assert_eq!(parsed_review["data"]["task_object_id"], task_oid);
    assert_eq!(parsed_review["data"]["next_status"], "accepted");

    let task_vid_after = parsed_review["data"]["task_version_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(task_vid_before, task_vid_after);

    // 3. Verify status in show
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
    assert_eq!(parsed_show["data"]["payload"]["status"], "accepted");

    // 4. Assert exactly one work item with this task_id in list
    let list_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("list")
        .arg("--include-closed")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_list: Value = serde_json::from_slice(&list_output).unwrap();
    let tasks = parsed_list["data"]["tasks"].as_array().unwrap();
    let matching_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| t["task_id"] == "review-head-update-task")
        .collect();

    assert_eq!(
        matching_tasks.len(),
        1,
        "Should only be one task object for the given task_id"
    );
}

#[test]
fn review_rejected_updates_existing_task_to_closed() {
    let (_dir, root) = setup_and_init_example();

    // 1. Ingest a work item
    let task_json_path = root.join("task.json");
    let payload = serde_json::json!({
        "task_id": "reject-task",
        "title": "Reject Test",
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

    // 2. Perform rejected review
    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("review")
        .arg(&task_oid)
        .arg("--decision")
        .arg("rejected")
        .assert()
        .success();

    // 3. Verify status
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
    assert_eq!(parsed_show["data"]["payload"]["status"], "rejected");
}

#[test]
fn test_terminal_tasks_hidden_from_default_list() {
    let (_dir, root) = setup_and_init_example();

    // 1. Ingest task
    let task_json_path = root.join("task.json");
    let payload = serde_json::json!({
        "task_id": "terminal-hide-task",
        "title": "Terminal Hide Test",
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

    // 2. Accept task
    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("review")
        .arg(&task_oid)
        .arg("--decision")
        .arg("accepted")
        .assert()
        .success();

    // 3. Verify hidden from default list
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
    let tasks = parsed_list["data"]["tasks"].as_array().unwrap();
    assert!(tasks.iter().all(|t| t["task_id"] != "terminal-hide-task"));

    // 4. Verify visible with --include-closed
    let list_incl_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("list")
        .arg("--include-closed")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_incl: Value = serde_json::from_slice(&list_incl_output).unwrap();
    let tasks_incl = parsed_incl["data"]["tasks"].as_array().unwrap();
    assert!(tasks_incl
        .iter()
        .any(|t| t["task_id"] == "terminal-hide-task"));
}

#[test]
fn test_legacy_closed_normalizes_to_completed() {
    let (_dir, root) = setup_and_init_example();

    let task_json_path = root.join("task.json");
    let payload = serde_json::json!({
        "task_id": "legacy-closed-task",
        "title": "Legacy Closed Test",
        "status": "closed"
    });
    fs::write(&task_json_path, serde_json::to_string(&payload).unwrap()).unwrap();

    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("ingest-task")
        .arg("--source")
        .arg("native-json")
        .arg(&task_json_path)
        .assert()
        .success();

    // Verify list --status closed works (mapped to completed)
    let list_output = workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("list")
        .arg("--status")
        .arg("closed")
        .arg("--include-closed")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed_list: Value = serde_json::from_slice(&list_output).unwrap();
    let tasks = parsed_list["data"]["tasks"].as_array().unwrap();
    assert!(tasks.iter().any(|t| t["task_id"] == "legacy-closed-task"));
    assert_eq!(tasks[0]["status"], "completed");
}

#[test]
fn test_needs_revision_produces_followup_required() {
    let (_dir, root) = setup_and_init_example();

    let task_json_path = root.join("task.json");
    let payload = serde_json::json!({
        "task_id": "revision-task",
        "title": "Revision Test",
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

    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("review")
        .arg(&task_oid)
        .arg("--decision")
        .arg("needs_revision")
        .assert()
        .success();

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
    let payload = &parsed_show["data"]["payload"];
    assert_eq!(payload["status"], "followup_required");
    assert_eq!(payload["kernel:process"], "active");
    assert_eq!(payload["kernel:review"], "needs_revision");
}

#[test]
fn test_closure_timeline_summary_is_populated() {
    let (_dir, root) = setup_and_init_example();

    let task_json_path = root.join("task.json");
    let payload = serde_json::json!({
        "task_id": "closure-summary-task",
        "title": "Closure Summary Test",
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

    workspace_command()
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("orchestration")
        .arg("review")
        .arg(&task_oid)
        .arg("--decision")
        .arg("accepted")
        .assert()
        .success();

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
    let events = parsed_timeline["data"]["events"].as_array().unwrap();
    let closure_event = events.iter().find(|e| e["class"] == "closure").unwrap();
    assert!(closure_event["summary"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("accepted"));
}
