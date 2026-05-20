use std::path::PathBuf;

use assert_cmd::Command;
use tempfile::tempdir;

fn write_test_declaration(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

fn init_workspace(dir: &std::path::Path) {
    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root").arg(dir).arg("init");
    cmd.assert().success();
}

#[test]
fn register_non_sensitive_declaration_no_auth() {
    let dir = tempdir().unwrap();
    init_workspace(dir.path());

    let decl = write_test_declaration(
        dir.path(),
        "instruction.md",
        r#"---
name: test_instruction
version: 0.2.0
description: Test instruction
purpose: test
input_classes:
  - source_note
output_classes:
  - finding
execution_policy: single
trace_policy: none
register: default
---
Hello world"#,
    );

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("declare")
        .arg("register")
        .arg("--kind")
        .arg("instruction")
        .arg(&decl);
    cmd.assert().success();
}

#[test]
fn register_sensitive_declaration_fails_when_unauthorized() {
    let dir = tempdir().unwrap();
    init_workspace(dir.path());

    let decl = write_test_declaration(
        dir.path(),
        "standing_policy.yaml",
        r#"name: test_policy
version: 0.2.0
description: Test
transition_rules: []
operation_requirements: []
escalations: []"#,
    );

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("declare")
        .arg("register")
        .arg("--kind")
        .arg("standing-policy")
        .arg(&decl)
        .env("EM_TRUSTED_ACTORS", "admin")
        .env("EM_ACTOR", "untrusted-user");
    cmd.assert().failure();
}

#[test]
fn register_sensitive_declaration_succeeds_when_authorized() {
    let dir = tempdir().unwrap();
    init_workspace(dir.path());

    let decl = write_test_declaration(
        dir.path(),
        "standing_policy.yaml",
        r#"name: test_policy
version: 0.2.0
description: Test
transition_rules: []
operation_requirements: []
escalations: []"#,
    );

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("declare")
        .arg("register")
        .arg("--kind")
        .arg("standing-policy")
        .arg(&decl)
        .env("EM_TRUSTED_ACTORS", "admin")
        .env("EM_ACTOR", "admin");
    cmd.assert().success();
}

#[test]
fn register_provider_profile_fails_when_unauthorized() {
    let dir = tempdir().unwrap();
    init_workspace(dir.path());

    let decl = write_test_declaration(
        dir.path(),
        "provider.yaml",
        r#"name: test_provider
version: 0.2.0
description: Test
provider: mock
model: echo
budget:
  max_input_tokens: 1000
  max_output_tokens: 1000
  max_latency_ms: 5000
allowed_operations: []
exposure:
  allow_prose_objects: true
  allow_structured_declarations: false
  allow_work_surface_only: false
  allow_export_requests: false
response_contract:
  format: text"#,
    );

    let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
    cmd.arg("--root")
        .arg(dir.path())
        .arg("declare")
        .arg("register")
        .arg("--kind")
        .arg("provider-profile")
        .arg(&decl)
        .env("EM_TRUSTED_ACTORS", "admin")
        .env("EM_ACTOR", "hacker");
    cmd.assert().failure();
}
