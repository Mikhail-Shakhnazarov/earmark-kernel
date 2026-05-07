use std::{fs, path::PathBuf};
use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn run_inspection_and_reports() {
    let dir = tempdir().unwrap();
    let workspace = workspace_root();
    
    // 1. Init
    Command::cargo_bin("earmark-cli").unwrap()
        .arg("--root").arg(dir.path())
        .arg("init")
        .assert().success();

    // 2. Register system from research synthesis example
    let manifest = workspace.join("examples/research-synthesis/declarations/systems/system.yaml");
    Command::cargo_bin("earmark-cli").unwrap()
        .arg("--root").arg(dir.path())
        .arg("system").arg("register").arg(&manifest)
        .assert().success();

    // 3. Activate system
    Command::cargo_bin("earmark-cli").unwrap()
        .arg("--root").arg(dir.path())
        .arg("system").arg("activate").arg("sys_research_synthesis")
        .assert().success();

    // 4. Deposit a source note
    let deposit_output = Command::cargo_bin("earmark-cli").unwrap()
        .arg("--root").arg(dir.path())
        .arg("--json")
        .arg("deposit")
        .arg("--class").arg("source_note")
        .arg("--title").arg("Test Note")
        .arg("--body").arg("This is a test note.")
        .assert().success().get_output().stdout.clone();
    
    let deposit_json: Value = serde_json::from_slice(&deposit_output).unwrap();
    let object_id = deposit_json["data"]["object_id"].as_str().unwrap();

    // 5. Run workflow
    let run_output = Command::cargo_bin("earmark-cli").unwrap()
        .arg("--root").arg(dir.path())
        .arg("--json")
        .arg("workflow").arg("run")
        .arg("research_synthesis")
        .arg("--system-id").arg("sys_research_synthesis")
        .arg("--with").arg(object_id)
        .assert().success().get_output().stdout.clone();

    let run_json: Value = serde_json::from_slice(&run_output).unwrap();
    let run_id = run_json["data"]["run_id"].as_str().unwrap();

    // 6. Test run explain
    let explain_output = Command::cargo_bin("earmark-cli").unwrap()
        .arg("--root").arg(dir.path())
        .arg("--json")
        .arg("run").arg("explain").arg(run_id)
        .assert().success().get_output().stdout.clone();
    let explain_json: Value = serde_json::from_slice(&explain_output).unwrap();
    assert_eq!(explain_json["data"]["ok"], true);
    assert_eq!(explain_json["data"]["kind"], "run");
    assert!(!explain_json["data"]["related"]["assignments"].as_array().unwrap().is_empty());

    // 7. Test run timeline
    let timeline_output = Command::cargo_bin("earmark-cli").unwrap()
        .arg("--root").arg(dir.path())
        .arg("--json")
        .arg("run").arg("timeline").arg(run_id)
        .assert().success().get_output().stdout.clone();
    let timeline_json: Value = serde_json::from_slice(&timeline_output).unwrap();
    assert_eq!(timeline_json["data"]["kind"], "run_timeline");
    assert!(!timeline_json["data"]["timeline"]["events"].as_array().unwrap().is_empty());

    // 8. Test run graph
    let graph_output = Command::cargo_bin("earmark-cli").unwrap()
        .arg("--root").arg(dir.path())
        .arg("--json")
        .arg("run").arg("graph").arg(run_id)
        .assert().success().get_output().stdout.clone();
    let graph_json: Value = serde_json::from_slice(&graph_output).unwrap();
    assert_eq!(graph_json["data"]["kind"], "run_graph");
    assert!(!graph_json["data"]["graph"]["nodes"].as_array().unwrap().is_empty());

    // 9. Test report generation
    let report_path = dir.path().join("reports/run_report.html");
    Command::cargo_bin("earmark-cli").unwrap()
        .arg("--root").arg(dir.path())
        .arg("--json")
        .arg("report").arg("run").arg(run_id)
        .arg("--output").arg(&report_path)
        .assert().success();
    assert!(report_path.exists());
    let report_html = fs::read_to_string(&report_path).unwrap();
    assert!(report_html.contains("Run Summary"));
    assert!(report_html.contains("Relationship Graph"));
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
