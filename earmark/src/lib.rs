use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum EarmarkError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("command failed: {0}")]
    Command(String),
    #[error("missing field '{0}' in cli response")]
    MissingField(&'static str),
}

#[derive(Debug, Clone)]
pub struct DepositedObject {
    pub object_id: String,
    pub version_id: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowRun {
    pub run_id: String,
}

#[derive(Debug, Clone)]
pub struct EarmarkWorkspace {
    root: PathBuf,
    default_system_id: Option<String>,
}

impl EarmarkWorkspace {
    pub fn open_or_init(path: impl AsRef<Path>) -> Result<Self, EarmarkError> {
        let root = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;
        let ws = Self {
            root,
            default_system_id: None,
        };
        ws.run_cli_json(["init"])?;
        Ok(ws)
    }

    pub fn register_system_from_path(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<(), EarmarkError> {
        let system_path = path.as_ref();
        self.run_cli_json([
            "system",
            "register",
            system_path.to_string_lossy().as_ref(),
        ])?;

        let content = std::fs::read_to_string(system_path)?;
        let parsed: serde_yaml::Value = serde_yaml::from_str(&content)?;
        if let Some(system_id) = parsed.get("system_id").and_then(|v| v.as_str()) {
            self.default_system_id = Some(system_id.to_string());
            self.run_cli_json(["system", "activate", system_id])?;
        }
        Ok(())
    }

    pub fn deposit_markdown(
        &self,
        class: &str,
        title: &str,
        body: &str,
    ) -> Result<DepositedObject, EarmarkError> {
        let out = self.run_cli_json([
            "deposit", "--class", class, "--title", title, "--body", body,
        ])?;
        let data = out.get("data").ok_or(EarmarkError::MissingField("data"))?;
        Ok(DepositedObject {
            object_id: data
                .get("object_id")
                .and_then(|v| v.as_str())
                .ok_or(EarmarkError::MissingField("data.object_id"))?
                .to_string(),
            version_id: data
                .get("version_id")
                .and_then(|v| v.as_str())
                .ok_or(EarmarkError::MissingField("data.version_id"))?
                .to_string(),
        })
    }

    pub fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<WorkflowRun, EarmarkError> {
        let mut args = vec!["workflow".to_string(), "run".to_string(), workflow_id.to_string()];
        if let Some(system_id) = &self.default_system_id {
            args.push("--system-id".to_string());
            args.push(system_id.clone());
        }
        for input in inputs {
            args.push("--with".to_string());
            args.push(input.as_ref().to_string());
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = self.run_cli_json_refs(&refs)?;
        let data = out.get("data").ok_or(EarmarkError::MissingField("data"))?;
        Ok(WorkflowRun {
            run_id: data
                .get("run_id")
                .and_then(|v| v.as_str())
                .ok_or(EarmarkError::MissingField("data.run_id"))?
                .to_string(),
        })
    }

    pub fn report_run(&self, run_id: &str) -> Result<String, EarmarkError> {
        let output = tempfile::Builder::new()
            .prefix("earmark-report-")
            .suffix(".html")
            .tempfile_in(&self.root)?;
        let path = output.path().to_path_buf();
        drop(output);

        self.run_cli_json([
            "report",
            "run",
            run_id,
            "--output",
            path.to_string_lossy().as_ref(),
        ])?;
        Ok(std::fs::read_to_string(path)?)
    }

    fn run_cli_json<const N: usize>(&self, args: [&str; N]) -> Result<Value, EarmarkError> {
        self.run_cli_json_refs(&args)
    }

    fn run_cli_json_refs(&self, args: &[&str]) -> Result<Value, EarmarkError> {
        let mut cmd = Command::new(resolve_cli_bin());
        cmd.arg("--root")
            .arg(&self.root)
            .arg("--json")
            .args(args);
        let out = cmd.output()?;
        if !out.status.success() {
            return Err(EarmarkError::Command(
                String::from_utf8_lossy(&out.stderr).trim().to_string(),
            ));
        }
        let value: Value = serde_json::from_slice(&out.stdout)?;
        if value.get("ok").and_then(|v| v.as_bool()) == Some(false) {
            return Err(EarmarkError::Command(
                value
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown cli error")
                    .to_string(),
            ));
        }
        Ok(value)
    }
}

fn resolve_cli_bin() -> String {
    std::env::var("EARMARK_CLI_BIN").unwrap_or_else(|_| "earmark-cli".to_string())
}
