use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_topology_demo_declarations_valid() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // 1. Init
    let status = Command::new("cargo")
        .args(["run", "--", "--root", root.to_str().unwrap(), "init"])
        .status()
        .unwrap();
    assert!(status.success());

    // 2. Register System
    let status = Command::new("cargo")
        .args([
            "run",
            "--",
            "--root",
            root.to_str().unwrap(),
            "declare",
            "register",
            "--kind",
            "system",
            "../examples/risk-assessment-demo/systems/system.yaml",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    // 3. Verify Workflow exists and has expected topology
    // We can use 'em query' or just check if it registered
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--root",
            root.to_str().unwrap(),
            "query",
            "--kind",
            "workflow",
        ])
        .output()
        .unwrap();
    assert!(status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("risk_assessment_workflow"));
}
