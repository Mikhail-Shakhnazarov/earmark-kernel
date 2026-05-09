use std::fs;
use std::process::Command;
use assert_cmd::prelude::*;

#[test]
fn test_documentation_drift() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap();
    let cli_md_path = workspace_root.join("docs/reference/cli.md");
    
    if !cli_md_path.exists() {
        panic!("CLI reference documentation not found at {:?}", cli_md_path);
    }

    let content = fs::read_to_string(cli_md_path).expect("failed to read cli.md");
    let mut documented_commands = Vec::new();

    for line in content.lines() {
        if line.starts_with("### `em ") {
            let cmd_part = line.trim_start_matches("### `em ").trim_end_matches('`');
            
            // Extract the base command and subcommands (parts before flags/placeholders)
            let base_parts: Vec<&str> = cmd_part.split_whitespace()
                .take_while(|p| !p.starts_with('-') && !p.starts_with('<') && !p.starts_with('['))
                .collect();
            
            if base_parts.is_empty() {
                continue;
            }
            let base_cmd = base_parts.join(" ");

            // Extract flags mentioned in the same heading
            let mut flags = Vec::new();
            for part in cmd_part.split_whitespace() {
                let clean_part = part.trim_matches(|c| c == '[' || c == ']');
                if clean_part.starts_with("--") {
                    flags.push(clean_part.to_string());
                }
            }

            documented_commands.push((base_cmd, flags));
        }
    }

    assert!(!documented_commands.is_empty(), "No documented commands found in cli.md");

    for (base_cmd, flags) in documented_commands {
        let parts: Vec<&str> = base_cmd.split_whitespace().collect();
        let mut cmd = Command::cargo_bin("earmark-cli").unwrap();
        
        for part in &parts {
            cmd.arg(part);
        }
        cmd.arg("--help");

        let output = cmd.output().expect("failed to execute command");
        let help_text = String::from_utf8_lossy(&output.stdout);
        
        assert!(
            output.status.success(),
            "Documented command 'em {}' does not exist or failed --help: {}",
            base_cmd,
            String::from_utf8_lossy(&output.stderr)
        );

        for flag in flags {
            assert!(
                help_text.contains(&flag),
                "Documented flag '{}' not found in help output for 'em {}'",
                flag,
                base_cmd
            );
        }
    }
}
