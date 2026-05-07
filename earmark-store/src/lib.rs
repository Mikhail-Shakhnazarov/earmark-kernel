use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use earmark_core::{
    to_json_pretty, Envelope, HeaderValue, Kind, ObjectId, ObjectRef, PayloadRef, Provenance,
    Standing, Timestamp, VersionId, VersionRef,
};
use git2::{IndexAddOption, Repository, Signature};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadEncoding {
    Json,
    Markdown,
    Yaml,
}

impl PayloadEncoding {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Markdown => "md",
            Self::Yaml => "yaml",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredPayload {
    pub format: PayloadEncoding,
    pub bytes: Vec<u8>,
}

impl StoredPayload {
    pub fn new(format: PayloadEncoding, bytes: Vec<u8>) -> Self {
        Self { format, bytes }
    }

    pub fn from_json_bytes(bytes: Vec<u8>) -> Self {
        Self::new(PayloadEncoding::Json, bytes)
    }

    pub fn from_markdown<S: Into<String>>(input: S) -> Self {
        Self::new(PayloadEncoding::Markdown, input.into().into_bytes())
    }

    pub fn from_yaml<S: Into<String>>(input: S) -> Self {
        Self::new(PayloadEncoding::Yaml, input.into().into_bytes())
    }

    pub fn payload_ref(&self) -> PayloadRef {
        PayloadRef::from_bytes(&self.bytes)
    }

    pub fn as_utf8(&self) -> Result<String, StoreError> {
        Ok(String::from_utf8(self.bytes.clone())?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryManifest {
    pub schema_version: String,
    pub initialized_at: Timestamp,
}

#[derive(Debug, Clone)]
pub struct StoredObject {
    pub envelope: Envelope,
    pub payload: StoredPayload,
}

impl StoredObject {
    pub fn new(
        kind: Kind,
        class: Option<String>,
        standing: Standing,
        provenance: Provenance,
        headers: BTreeMap<String, HeaderValue>,
        payload: StoredPayload,
        parents: Vec<VersionRef>,
    ) -> Self {
        let now = Utc::now();
        let payload_ref = payload.payload_ref();
        Self {
            envelope: Envelope {
                id: ObjectId::new(),
                version_id: VersionId::new(),
                kind,
                class,
                standing,
                provenance,
                headers,
                payload_ref,
                parents,
                created_at: now,
                updated_at: now,
            },
            payload,
        }
    }

    pub fn with_parent(
        previous: &StoredObject,
        standing: Standing,
        headers: BTreeMap<String, HeaderValue>,
        payload: StoredPayload,
    ) -> Self {
        let now = Utc::now();
        let payload_ref = payload.payload_ref();
        Self {
            envelope: Envelope {
                id: previous.envelope.id.clone(),
                version_id: VersionId::new(),
                kind: previous.envelope.kind.clone(),
                class: previous.envelope.class.clone(),
                standing,
                provenance: previous.envelope.provenance.clone(),
                headers,
                payload_ref,
                parents: vec![previous.envelope.version_ref()],
                created_at: previous.envelope.created_at,
                updated_at: now,
            },
            payload,
        }
    }

    pub fn object_ref(&self) -> ObjectRef {
        self.envelope.object_ref()
    }
}

#[derive(Debug, Clone)]
pub struct BatchWrite {
    pub message: String,
    pub objects: Vec<StoredObject>,
}

pub trait CanonicalStore {
    fn root(&self) -> &Path;
    fn init_layout(&self) -> Result<(), StoreError>;
    fn write_object(&self, object: &StoredObject) -> Result<VersionRef, StoreError>;
    fn write_batch(&self, batch: &BatchWrite) -> Result<Vec<VersionRef>, StoreError>;
    fn read_version(&self, version: &VersionRef) -> Result<StoredObject, StoreError>;
    fn read_head(&self, object_id: &ObjectId) -> Result<Option<StoredObject>, StoreError>;
    fn read_head_ref(&self, object_id: &ObjectId) -> Result<Option<VersionRef>, StoreError>;
    fn list_versions(&self, object_id: &ObjectId) -> Result<Vec<VersionRef>, StoreError>;
    fn resolve_payload(&self, payload_ref: &PayloadRef) -> Result<StoredPayload, StoreError>;
    fn scan_objects(&self) -> Result<Vec<StoredObject>, StoreError>;
    fn version_path(&self, version: &VersionRef) -> PathBuf;
}

#[derive(Debug, Clone)]
pub struct GitCanonicalStore {
    root: PathBuf,
}

impl GitCanonicalStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn canonical_dir(&self) -> PathBuf {
        self.root.join(".earmark").join("canonical")
    }

    pub fn objects_dir(&self) -> PathBuf {
        self.canonical_dir().join("objects")
    }

    pub fn payloads_dir(&self) -> PathBuf {
        self.canonical_dir().join("payloads")
    }

    pub fn heads_dir(&self) -> PathBuf {
        self.canonical_dir().join("heads")
    }

    pub fn derived_dir(&self) -> PathBuf {
        self.root.join(".earmark").join("derived")
    }

    pub fn work_surfaces_dir(&self) -> PathBuf {
        self.root.join(".earmark").join("work_surfaces")
    }

    pub fn declarations_dir(&self) -> PathBuf {
        self.root.join(".earmark").join("declarations")
    }

    pub fn corpus_dir(&self) -> PathBuf {
        self.root.join("corpus")
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(".earmark").join("repository_manifest.json")
    }

    fn version_dir(&self, object_id: &ObjectId, version_id: &VersionId) -> PathBuf {
        self.objects_dir().join(&object_id.0).join(&version_id.0)
    }

    fn head_path(&self, object_id: &ObjectId) -> PathBuf {
        self.heads_dir().join(format!("{}.json", object_id.0))
    }

    fn payload_path(&self, payload_ref: &PayloadRef, encoding: PayloadEncoding) -> PathBuf {
        let safe = payload_ref.0.replace(':', "_");
        self.payloads_dir()
            .join(format!("{}.{}", safe, encoding.extension()))
    }

    fn ensure_repo(&self) -> Result<Repository, StoreError> {
        match Repository::open(&self.root) {
            Ok(repo) => Ok(repo),
            Err(_) => Ok(Repository::init(&self.root)?),
        }
    }

    fn commit(&self, message: &str) -> Result<(), StoreError> {
        let repo = self.ensure_repo()?;
        let mut index = repo.index()?;
        index.add_all([".earmark", "corpus"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let sig = Signature::now("earmark", "earmark@local")?;
        match repo.head() {
            Ok(head) => {
                let parent = head.peel_to_commit()?;
                repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?;
            }
            Err(_) => {
                repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])?;
            }
        }
        Ok(())
    }

    fn infer_encoding(path: &Path) -> Result<PayloadEncoding, StoreError> {
        match path.extension().and_then(|s| s.to_str()) {
            Some("json") => Ok(PayloadEncoding::Json),
            Some("md") => Ok(PayloadEncoding::Markdown),
            Some("yaml") | Some("yml") => Ok(PayloadEncoding::Yaml),
            other => Err(StoreError::UnknownPayloadEncoding(format!(
                "unsupported extension: {:?}",
                other
            ))),
        }
    }
}

impl CanonicalStore for GitCanonicalStore {
    fn root(&self) -> &Path {
        &self.root
    }

    fn init_layout(&self) -> Result<(), StoreError> {
        fs::create_dir_all(self.objects_dir())?;
        fs::create_dir_all(self.payloads_dir())?;
        fs::create_dir_all(self.heads_dir())?;
        fs::create_dir_all(self.derived_dir())?;
        fs::create_dir_all(self.work_surfaces_dir())?;
        fs::create_dir_all(self.corpus_dir())?;

        for subdir in [
            "classes",
            "instructions",
            "standing_policies",
            "workflows",
            "compiled_contexts",
            "provider_profiles",
            "systems",
        ] {
            fs::create_dir_all(self.declarations_dir().join(subdir))?;
        }

        if !self.manifest_path().exists() {
            let manifest = RepositoryManifest {
                schema_version: "0.1.0".to_string(),
                initialized_at: Utc::now(),
            };
            if let Some(parent) = self.manifest_path().parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(self.manifest_path(), to_json_pretty(&manifest)?)?;
        }

        let _ = self.ensure_repo()?;
        Ok(())
    }

    fn write_object(&self, object: &StoredObject) -> Result<VersionRef, StoreError> {
        self.write_batch(&BatchWrite {
            message: format!(
                "deposit {} {}",
                object.envelope.kind.as_str(),
                object.envelope.version_id.0
            ),
            objects: vec![object.clone()],
        })?
        .into_iter()
        .next()
        .ok_or_else(|| StoreError::Invariant("batch write returned no refs".to_string()))
    }

    fn write_batch(&self, batch: &BatchWrite) -> Result<Vec<VersionRef>, StoreError> {
        self.init_layout()?;
        let mut written = Vec::with_capacity(batch.objects.len());

        for object in &batch.objects {
            if object.payload.payload_ref() != object.envelope.payload_ref {
                return Err(StoreError::PayloadRefMismatch);
            }

            let payload_path =
                self.payload_path(&object.envelope.payload_ref, object.payload.format);
            if !payload_path.exists() {
                fs::write(&payload_path, &object.payload.bytes)?;
            }

            let version_dir = self.version_dir(&object.envelope.id, &object.envelope.version_id);
            fs::create_dir_all(&version_dir)?;
            fs::write(
                version_dir.join("envelope.json"),
                to_json_pretty(&object.envelope)?,
            )?;
            fs::write(
                version_dir.join(format!("payload.{}", object.payload.format.extension())),
                &object.payload.bytes,
            )?;
            fs::write(
                version_dir.join("payload_ref.txt"),
                object.envelope.payload_ref.0.as_bytes(),
            )?;
            fs::write(
                self.head_path(&object.envelope.id),
                to_json_pretty(&object.envelope.version_ref())?,
            )?;
            written.push(object.envelope.version_ref());
        }

        self.commit(&batch.message)?;
        Ok(written)
    }

    fn read_version(&self, version: &VersionRef) -> Result<StoredObject, StoreError> {
        let version_dir = self.version_dir(&version.id, &version.version_id);
        let env_path = version_dir.join("envelope.json");
        let envelope: Envelope = serde_json::from_slice(&fs::read(env_path)?)?;

        let payload_path = fs::read_dir(&version_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.starts_with("payload."))
                    .unwrap_or(false)
            })
            .ok_or_else(|| StoreError::MissingPayload(version.version_id.0.clone()))?;

        let format = Self::infer_encoding(&payload_path)?;
        let payload = StoredPayload::new(format, fs::read(payload_path)?);
        Ok(StoredObject { envelope, payload })
    }

    fn read_head(&self, object_id: &ObjectId) -> Result<Option<StoredObject>, StoreError> {
        match self.read_head_ref(object_id)? {
            Some(reference) => Ok(Some(self.read_version(&reference)?)),
            None => Ok(None),
        }
    }

    fn read_head_ref(&self, object_id: &ObjectId) -> Result<Option<VersionRef>, StoreError> {
        let path = self.head_path(object_id);
        if !path.exists() {
            return Ok(None);
        }
        let reference = serde_json::from_slice(&fs::read(path)?)?;
        Ok(Some(reference))
    }

    fn list_versions(&self, object_id: &ObjectId) -> Result<Vec<VersionRef>, StoreError> {
        let object_dir = self.objects_dir().join(&object_id.0);
        if !object_dir.exists() {
            return Ok(vec![]);
        }
        let mut refs = Vec::new();
        for entry in fs::read_dir(object_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let version_id = entry.file_name().to_string_lossy().to_string();
                refs.push(VersionRef::new(object_id.clone(), VersionId(version_id)));
            }
        }
        refs.sort_by(|a, b| a.version_id.0.cmp(&b.version_id.0));
        Ok(refs)
    }

    fn resolve_payload(&self, payload_ref: &PayloadRef) -> Result<StoredPayload, StoreError> {
        for extension in [
            PayloadEncoding::Json,
            PayloadEncoding::Markdown,
            PayloadEncoding::Yaml,
        ] {
            let path = self.payload_path(payload_ref, extension);
            if path.exists() {
                return Ok(StoredPayload::new(extension, fs::read(path)?));
            }
        }
        Err(StoreError::MissingPayload(payload_ref.0.clone()))
    }

    fn scan_objects(&self) -> Result<Vec<StoredObject>, StoreError> {
        let mut objects = Vec::new();
        for entry in WalkDir::new(self.objects_dir())
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.file_name() == "envelope.json" {
                let envelope: Envelope = serde_json::from_slice(&fs::read(entry.path())?)?;
                let version_dir = entry.path().parent().ok_or_else(|| {
                    StoreError::Invariant("envelope file has no version directory".to_string())
                })?;
                let payload_path = fs::read_dir(version_dir)?
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .find(|path| {
                        path.file_name()
                            .and_then(|s| s.to_str())
                            .map(|s| s.starts_with("payload."))
                            .unwrap_or(false)
                    })
                    .ok_or_else(|| StoreError::MissingPayload(envelope.version_id.0.clone()))?;
                let payload = StoredPayload::new(
                    Self::infer_encoding(&payload_path)?,
                    fs::read(payload_path)?,
                );
                objects.push(StoredObject { envelope, payload });
            }
        }
        Ok(objects)
    }

    fn version_path(&self, version: &VersionRef) -> PathBuf {
        self.version_dir(&version.id, &version.version_id)
    }
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("core error: {0}")]
    Core(#[from] earmark_core::CoreError),
    #[error("payload ref mismatch between envelope and stored bytes")]
    PayloadRefMismatch,
    #[error("missing payload for {0}")]
    MissingPayload(String),
    #[error("unknown payload encoding: {0}")]
    UnknownPayloadEncoding(String),
    #[error("invariant violation: {0}")]
    Invariant(String),
}
