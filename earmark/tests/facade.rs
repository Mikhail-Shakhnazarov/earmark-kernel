use earmark::{CliBackedWorkspace, EarmarkWorkspace};
use std::path::PathBuf;

#[test]
fn in_process_workspace_workflow_roundtrip_without_cli_path() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut workspace = EarmarkWorkspace::open_or_init(temp.path()).expect("open_or_init");

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let system_manifest =
        repo_root.join("examples/research-synthesis/declarations/systems/system.yaml");
    workspace
        .register_system_from_path(&system_manifest)
        .expect("register system");

    let note = workspace
        .deposit_markdown("source_note", "Facade Test Note", "A test note body")
        .expect("deposit markdown");

    let run = workspace
        .run_workflow("research_synthesis", [note.object_id.as_str()])
        .expect("run workflow");

    let report = workspace.report_run(&run.run_id).expect("report run");
    assert!(report.contains("Run Summary"));
    assert!(report.contains("Relationship Graph"));
}

#[test]
fn cli_backed_workspace_preserves_json_error_details() {
    let previous_cli_bin = std::env::var_os("EARMARK_CLI_BIN");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let fallback_bin = repo_root.join("target/debug/earmark-cli");
    if !fallback_bin.exists() {
        let status = std::process::Command::new("cargo")
            .arg("build")
            .arg("-p")
            .arg("earmark-cli")
            .current_dir(&repo_root)
            .status()
            .expect("cargo build should run");
        assert!(status.success(), "cargo build -p earmark-cli failed");
    }
    std::env::set_var("EARMARK_CLI_BIN", fallback_bin.as_os_str());

    let temp = tempfile::tempdir().expect("tempdir");
    let mut workspace = CliBackedWorkspace::open_or_init(temp.path()).expect("open_or_init");

    let system_manifest =
        repo_root.join("examples/research-synthesis/declarations/systems/system.yaml");
    workspace
        .register_system_from_path(&system_manifest)
        .expect("register system");

    let note = workspace
        .deposit_markdown("source_note", "Facade Test Note", "A test note body")
        .expect("deposit markdown");

    let _run = workspace
        .run_workflow("research_synthesis", [note.object_id.as_str()])
        .expect("run workflow");

    let err = workspace
        .run_workflow("non_existent_workflow", [note.object_id.as_str()])
        .expect_err("should fail");
    let msg = err.to_string();
    assert!(msg.contains("command failed"));
    assert!(msg.contains("not_found"));

    match previous_cli_bin {
        Some(value) => std::env::set_var("EARMARK_CLI_BIN", value),
        None => std::env::remove_var("EARMARK_CLI_BIN"),
    }
}
