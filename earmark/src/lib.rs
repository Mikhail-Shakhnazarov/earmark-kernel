//! Earmark workspace APIs.
//!
//! - `EarmarkWorkspace` is the primary in-process API.
//! - `CliBackedWorkspace` is a compatibility wrapper that shells out to `earmark-cli`.

use earmark_core::{
    to_yaml, HeaderValue, Kind, ObjectId, Provenance, RunRecord, RuntimeProvenance, Standing,
    VersionId, VersionRef,
};
use earmark_declarations::{
    activate_system_definition, load_class_definition, load_compiled_context_template,
    load_instruction, load_provider_profile, load_standing_policy, load_system_definition,
    load_workflow_definition, validate_class_definition, validate_compiled_context_template,
    validate_instruction, validate_provider_profile, validate_standing_policy,
    validate_system_definition, validate_workflow_definition,
};
use earmark_exec::{
    default_provider_registry, persistence_helpers::write_object_and_index, WorkflowRunRequest,
};
use earmark_index::{DerivedIndex, QueryFilter};
use earmark_runtime_tools::{DepositValidationContext, RuntimeToolSurface};
use earmark_store::{GitCanonicalStore, ObjectStore, StoredObject, StoredPayload, WorkspaceLayout};
use serde_json::Value;
use std::collections::BTreeMap;
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
    #[error("store error: {0}")]
    Store(#[from] earmark_store::StoreError),
    #[error("index error: {0}")]
    Index(#[from] earmark_index::IndexError),
    #[error("derive error: {0}")]
    Derive(#[from] earmark_declarations::DeriveError),
    #[error("runtime error: {0}")]
    Runtime(#[from] earmark_runtime_tools::RuntimeToolError),
    #[error("exec error: {0}")]
    Exec(#[from] earmark_exec::ExecError),
    #[error("core error: {0}")]
    Core(#[from] earmark_core::CoreError),
    #[error("not found: {0}")]
    NotFound(String),
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

pub struct EarmarkWorkspace {
    root: PathBuf,
    store: GitCanonicalStore,
    index: DerivedIndex,
    provider_registry: earmark_exec::ProviderRegistry,
    default_system_id: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct PathSystemManifest {
    schema: Option<String>,
    system_id: String,
    namespace: String,
    title: String,
    description: Option<String>,
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    instructions: Vec<String>,
    #[serde(default)]
    standing_policies: Vec<String>,
    #[serde(default)]
    compiled_contexts: Vec<String>,
    #[serde(default)]
    provider_profiles: Vec<String>,
    #[serde(default)]
    workflows: Vec<String>,
    default_compiled_context: Option<String>,
    default_provider_profile: Option<String>,
    runtime_profile: earmark_core::RuntimeProfile,
}

impl EarmarkWorkspace {
    pub fn open_or_init(path: impl AsRef<Path>) -> Result<Self, EarmarkError> {
        let root = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;

        let store = GitCanonicalStore::new(&root);
        store.init_layout()?;
        let index = DerivedIndex::open(&root)?;

        Ok(Self {
            root,
            store,
            index,
            provider_registry: default_provider_registry(),
            default_system_id: None,
        })
    }

    fn surface(&self) -> RuntimeToolSurface<'_, GitCanonicalStore> {
        RuntimeToolSurface::new(&self.store, &self.index, &self.provider_registry)
    }

    pub fn register_system_from_path(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<(), EarmarkError> {
        let system_path = path.as_ref();
        let system = self.load_system_definition_any(system_path)?;
        validate_system_definition(&self.store, &system)?;

        let mut headers = BTreeMap::new();
        headers.insert(
            "title".to_string(),
            HeaderValue::String(system.title.clone()),
        );

        let object = StoredObject::new(
            Kind::SystemDefinition,
            Some("system_definition".to_string()),
            Standing::default(),
            Provenance::direct_input("earmark"),
            headers,
            StoredPayload::from_yaml(to_yaml(&system)?),
            vec![],
        );
        write_object_and_index(&self.store, &self.index, &object)?;

        let active = activate_system_definition(&self.store, &self.index, &system.system_id)?;
        self.default_system_id = Some(active.system_id);
        Ok(())
    }

    fn load_system_definition_any(
        &self,
        system_path: &Path,
    ) -> Result<earmark_core::SystemDefinition, EarmarkError> {
        let content = std::fs::read_to_string(system_path)?;
        let raw: serde_yaml::Value = serde_yaml::from_str(&content)?;
        let schema = raw.get("schema").and_then(|v| v.as_str());
        if schema == Some("earmark.path_system_manifest.v1") {
            let manifest: PathSystemManifest = serde_yaml::from_str(&content)?;
            return self.register_path_manifest(system_path, manifest);
        }
        Ok(load_system_definition(system_path)?)
    }

    fn register_path_manifest(
        &self,
        manifest_path: &Path,
        manifest: PathSystemManifest,
    ) -> Result<earmark_core::SystemDefinition, EarmarkError> {
        let _ = manifest.schema.as_deref();
        let mut registry: BTreeMap<PathBuf, VersionRef> = BTreeMap::new();

        let classes = self.register_many(
            manifest_path,
            &manifest.classes,
            Kind::Object,
            Some("class_definition"),
            |p| {
                let d = load_class_definition(p)?;
                validate_class_definition(&d)?;
                Ok((d.name.clone(), StoredPayload::from_yaml(to_yaml(&d)?)))
            },
            &mut registry,
        )?;

        let instructions = self.register_many(
            manifest_path,
            &manifest.instructions,
            Kind::Instruction,
            None,
            |p| {
                let d = load_instruction(p)?;
                validate_instruction(&d)?;
                let raw = std::fs::read_to_string(p)?;
                Ok((d.name.clone(), StoredPayload::from_markdown(raw)))
            },
            &mut registry,
        )?;

        let standing_policies = self.register_many(
            manifest_path,
            &manifest.standing_policies,
            Kind::Policy,
            None,
            |p| {
                let d = load_standing_policy(p)?;
                validate_standing_policy(&d)?;
                Ok((d.name.clone(), StoredPayload::from_yaml(to_yaml(&d)?)))
            },
            &mut registry,
        )?;

        let compiled_contexts = self.register_many(
            manifest_path,
            &manifest.compiled_contexts,
            Kind::CompiledContextTemplate,
            None,
            |p| {
                let d = load_compiled_context_template(p)?;
                validate_compiled_context_template(&d)?;
                Ok((d.name.clone(), StoredPayload::from_yaml(to_yaml(&d)?)))
            },
            &mut registry,
        )?;

        let provider_profiles = self.register_many(
            manifest_path,
            &manifest.provider_profiles,
            Kind::ProviderProfile,
            None,
            |p| {
                let d = load_provider_profile(p)?;
                validate_provider_profile(&d)?;
                Ok((d.name.clone(), StoredPayload::from_yaml(to_yaml(&d)?)))
            },
            &mut registry,
        )?;

        let mut workflows = Vec::new();
        for rel in &manifest.workflows {
            let p = resolve_manifest_path(manifest_path, rel);
            let d = load_workflow_definition(&p)?;
            validate_workflow_definition(&d)?;
            let resolved = resolve_workflow_paths(&p, d, &registry)?;
            let mut headers = BTreeMap::new();
            headers.insert(
                "title".to_string(),
                HeaderValue::String(resolved.name.clone()),
            );
            let object = StoredObject::new(
                Kind::Workflow,
                None,
                Standing::default(),
                Provenance::direct_input("earmark"),
                headers,
                StoredPayload::from_yaml(to_yaml(&resolved)?),
                vec![],
            );
            let vref = write_object_and_index(&self.store, &self.index, &object)?;
            registry.insert(canonicalized(&p), vref.clone());
            workflows.push(vref);
        }

        let system = earmark_core::SystemDefinition {
            system_id: manifest.system_id,
            namespace: manifest.namespace,
            title: manifest.title,
            description: manifest.description,
            classes,
            instructions,
            policies: standing_policies,
            workflows,
            compiled_contexts,
            provider_profiles,
            default_compiled_context: manifest
                .default_compiled_context
                .as_deref()
                .map(|v| resolve_ref(manifest_path, v, &registry))
                .transpose()?,
            default_provider_profile: manifest
                .default_provider_profile
                .as_deref()
                .map(|v| resolve_ref(manifest_path, v, &registry))
                .transpose()?,
            runtime_profile: manifest.runtime_profile,
            standing_dimensions: vec![],
            activated_at: None,
        };
        Ok(system)
    }

    fn register_many<F>(
        &self,
        manifest_path: &Path,
        paths: &[String],
        kind: Kind,
        class: Option<&str>,
        mut load_payload: F,
        registry: &mut BTreeMap<PathBuf, VersionRef>,
    ) -> Result<Vec<VersionRef>, EarmarkError>
    where
        F: FnMut(&Path) -> Result<(String, StoredPayload), EarmarkError>,
    {
        let mut out = Vec::new();
        for rel in paths {
            let p = resolve_manifest_path(manifest_path, rel);
            let (name, payload) = load_payload(&p)?;
            let mut headers = BTreeMap::new();
            headers.insert("title".to_string(), HeaderValue::String(name));
            let object = StoredObject::new(
                kind.clone(),
                class.map(str::to_string),
                Standing::default(),
                Provenance::direct_input("earmark"),
                headers,
                payload,
                vec![],
            );
            let vref = write_object_and_index(&self.store, &self.index, &object)?;
            registry.insert(canonicalized(&p), vref.clone());
            out.push(vref);
        }
        Ok(out)
    }

    pub fn deposit_markdown(
        &self,
        class: &str,
        title: &str,
        body: &str,
    ) -> Result<DepositedObject, EarmarkError> {
        let object = self.surface().deposit_object(
            class.to_string(),
            Some("object".to_string()),
            Some(title.to_string()),
            serde_json::json!(body),
            RuntimeProvenance {
                actor: "earmark".to_string(),
                source_type: "in_process_facade".to_string(),
            },
            DepositValidationContext::default(),
        )?;

        Ok(DepositedObject {
            object_id: object.id.as_str().to_string(),
            version_id: object.version_id.as_str().to_string(),
        })
    }

    pub fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<WorkflowRun, EarmarkError> {
        let system_id = self
            .default_system_id
            .as_ref()
            .ok_or_else(|| EarmarkError::NotFound("no default system is active".to_string()))?;

        let system_definition = self
            .index
            .resolve_system_definition_symbolic_latest(system_id)?
            .ok_or_else(|| EarmarkError::NotFound(format!("system not found: {system_id}")))?;

        let workflow = self
            .index
            .resolve_workflow_symbolic_latest(workflow_id)?
            .ok_or_else(|| EarmarkError::NotFound(format!("workflow not found: {workflow_id}")))?;

        let mut input_refs = Vec::new();
        for input in inputs {
            let object_id = ObjectId::parse(input.as_ref().to_string())?;
            let version = self.store.read_head_ref(&object_id)?.ok_or_else(|| {
                EarmarkError::NotFound(format!(
                    "input object head not found: {}",
                    object_id.as_str()
                ))
            })?;
            let loaded = self.store.read_version(&version)?;
            input_refs.push(loaded.envelope.object_ref());
        }

        let req = WorkflowRunRequest {
            run_id: format!("run_{}", uuid_like()),
            system_definition,
            workflow,
            inputs: input_refs,
            handoff_manifest: None,
            transition_assignment: None,
            operator_approved: false,
        };

        let out = self.surface().run_workflow(req)?;
        Ok(WorkflowRun {
            run_id: out.record.run_id,
        })
    }

    /// Returns a compact in-process HTML summary for a run.
    ///
    /// This is intentionally lighter than the full CLI report renderer.
    pub fn report_run(&self, run_id: &str) -> Result<String, EarmarkError> {
        let record = self.find_run_record(run_id)?;
        let status = serde_json::to_string(&record.status)?
            .trim_matches('\"')
            .to_owned();
        let html = format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>Run {run_id}</title></head><body><h2>Run Summary</h2><p><strong>Run ID:</strong> {run_id}</p><p><strong>Status:</strong> {status}</p><p><strong>Workflow:</strong> {workflow}</p><h2>Relationship Graph</h2><p>Graph rendering is available in the CLI report surface.</p></body></html>",
            run_id = record.run_id,
            status = status,
            workflow = record.workflow.id.as_str(),
        );
        Ok(html)
    }

    fn find_run_record(&self, run_id: &str) -> Result<RunRecord, EarmarkError> {
        let rows = self.index.query_objects(&QueryFilter {
            kind: Some(Kind::RunRecord.as_str().to_string()),
            ..QueryFilter::default()
        })?;

        for row in rows {
            let version_ref = VersionRef::new(
                ObjectId::parse(row.object_id)?,
                VersionId::parse(row.version_id)?,
            );
            let loaded = self.store.read_version(&version_ref)?;
            let record: RunRecord = serde_json::from_slice(&loaded.payload.bytes)?;
            if record.run_id == run_id {
                return Ok(record);
            }
        }
        Err(EarmarkError::NotFound(format!("run not found: {run_id}")))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn canonicalized(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn resolve_manifest_path(manifest_path: &Path, rel: &str) -> PathBuf {
    manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(rel)
}

fn resolve_ref(
    manifest_path: &Path,
    rel: &str,
    registry: &BTreeMap<PathBuf, VersionRef>,
) -> Result<VersionRef, EarmarkError> {
    let p = canonicalized(&resolve_manifest_path(manifest_path, rel));
    registry.get(&p).cloned().ok_or_else(|| {
        EarmarkError::NotFound(format!("unresolved path reference: {}", p.display()))
    })
}

fn resolve_workflow_paths(
    workflow_path: &Path,
    decl: earmark_core::WorkflowDeclaration,
    registry: &BTreeMap<PathBuf, VersionRef>,
) -> Result<earmark_core::WorkflowDefinition, EarmarkError> {
    use earmark_core::{FlexibleVersionRef, WorkflowOperation};

    let resolve_flexible =
        |v: Option<FlexibleVersionRef>| -> Result<Option<VersionRef>, EarmarkError> {
            match v {
                Some(FlexibleVersionRef::Ref(r)) => Ok(Some(r)),
                Some(FlexibleVersionRef::Path(p)) => {
                    let abs = canonicalized(
                        &workflow_path
                            .parent()
                            .unwrap_or_else(|| Path::new("."))
                            .join(p),
                    );
                    Ok(Some(registry.get(&abs).cloned().ok_or_else(|| {
                        EarmarkError::NotFound(format!(
                            "workflow path reference not registered: {}",
                            abs.display()
                        ))
                    })?))
                }
                None => Ok(None),
            }
        };

    let operations = decl
        .operations
        .into_iter()
        .map(|op| -> Result<WorkflowOperation, EarmarkError> {
            Ok(WorkflowOperation {
                id: op.id,
                kind: op.kind,
                input_contracts: op.input_contracts,
                output_contracts: op.output_contracts,
                instruction: resolve_flexible(op.instruction)?,
                compiled_context: resolve_flexible(op.compiled_context)?,
                policy: resolve_flexible(op.policy)?,
                provider_profile: resolve_flexible(op.provider_profile)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(earmark_core::WorkflowDefinition {
        name: decl.name,
        version: decl.version,
        description: decl.description,
        operations,
        edges: decl.edges,
        guards: decl.guards,
        output_contracts: decl.output_contracts,
    })
}

#[derive(Debug, Clone)]
pub struct CliBackedWorkspace {
    root: PathBuf,
    default_system_id: Option<String>,
}

impl CliBackedWorkspace {
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
        self.run_cli_json(["system", "register", system_path.to_string_lossy().as_ref()])?;

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
        let mut args = vec![
            "workflow".to_string(),
            "run".to_string(),
            workflow_id.to_string(),
        ];
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
        cmd.arg("--root").arg(&self.root).arg("--json").args(args);
        let out = cmd.output()?;
        if !out.status.success() {
            if let Ok(value) = serde_json::from_slice::<Value>(&out.stdout) {
                return Err(EarmarkError::Command(cli_error_message(&value)));
            }
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(EarmarkError::Command(if stderr.is_empty() {
                format!("earmark-cli exited with status {}", out.status)
            } else {
                stderr
            }));
        }
        let value: Value = serde_json::from_slice(&out.stdout)?;
        if value.get("ok").and_then(|v| v.as_bool()) == Some(false) {
            return Err(EarmarkError::Command(cli_error_message(&value)));
        }
        Ok(value)
    }
}

fn cli_error_message(value: &Value) -> String {
    let message = value
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or("unknown cli error");
    match value
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_str())
    {
        Some(code) if !code.trim().is_empty() => format!("{}: {}", code, message),
        _ => message.to_string(),
    }
}

fn resolve_cli_bin() -> String {
    std::env::var("EARMARK_CLI_BIN").unwrap_or_else(|_| "earmark-cli".to_string())
}

fn uuid_like() -> String {
    let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    format!("{:x}", ts)
}
