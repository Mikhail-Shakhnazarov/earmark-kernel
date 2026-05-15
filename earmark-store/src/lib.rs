use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use fs4::fs_std::FileExt;

mod backend;

use crate::backend::{GitBackend, GixBackend};
use chrono::Utc;
use earmark_core::{
    to_json_pretty, Envelope, HeaderValue, Kind, ObjectId, ObjectRef, PayloadRef, Provenance,
    Standing, Timestamp, VersionId, VersionRef,
};
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

    pub fn as_utf8_str(&self) -> Result<&str, StoreError> {
        Ok(std::str::from_utf8(&self.bytes)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryManifest {
    pub schema_version: String,
    pub initialized_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceLayoutStatus {
    pub root_exists: bool,
    pub git_exists: bool,
    pub manifest_exists: bool,
    pub canonical_dir_exists: bool,
    pub objects_dir_exists: bool,
    pub payloads_dir_exists: bool,
    pub heads_dir_exists: bool,
    pub derived_dir_exists: bool,
    pub work_surfaces_dir_exists: bool,
    pub declarations_dir_exists: bool,
    pub corpus_dir_exists: bool,
}

impl WorkspaceLayoutStatus {
    pub fn is_initialized(&self) -> bool {
        self.root_exists
            && self.git_exists
            && self.manifest_exists
            && self.canonical_dir_exists
            && self.objects_dir_exists
            && self.payloads_dir_exists
            && self.heads_dir_exists
            && self.derived_dir_exists
            && self.work_surfaces_dir_exists
            && self.declarations_dir_exists
            && self.corpus_dir_exists
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredObject {
    pub envelope: Envelope,
    pub payload: StoredPayload,
}

pub struct StoredObjectBuilder {
    id: Option<ObjectId>,
    kind: Kind,
    class: Option<String>,
    standing: Standing,
    provenance: Option<Provenance>,
    headers: BTreeMap<String, HeaderValue>,
    payload: StoredPayload,
    parents: Vec<VersionRef>,
}

impl StoredObjectBuilder {
    pub fn id(mut self, id: ObjectId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    pub fn standing(mut self, standing: Standing) -> Self {
        self.standing = standing;
        self
    }

    pub fn provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<HeaderValue>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn headers(mut self, headers: BTreeMap<String, HeaderValue>) -> Self {
        self.headers = headers;
        self
    }

    pub fn parents(mut self, parents: Vec<VersionRef>) -> Self {
        self.parents = parents;
        self
    }

    pub fn build(self) -> Result<StoredObject, String> {
        let provenance = self
            .provenance
            .ok_or_else(|| "provenance is required".to_string())?;

        Ok(StoredObject::new_with_id(
            self.id.unwrap_or_default(),
            self.kind,
            self.class,
            self.standing,
            provenance,
            self.headers,
            self.payload,
            self.parents,
        ))
    }
}

impl StoredObject {
    pub fn builder(kind: Kind, payload: StoredPayload) -> StoredObjectBuilder {
        StoredObjectBuilder {
            id: None,
            kind,
            class: None,
            standing: Standing::default(),
            provenance: None,
            headers: BTreeMap::new(),
            payload,
            parents: Vec::new(),
        }
    }

    pub fn new(
        kind: Kind,
        class: Option<String>,
        standing: Standing,
        provenance: Provenance,
        headers: BTreeMap<String, HeaderValue>,
        payload: StoredPayload,
        parents: Vec<VersionRef>,
    ) -> Self {
        Self::new_with_id(
            ObjectId::new(),
            kind,
            class,
            standing,
            provenance,
            headers,
            payload,
            parents,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_id(
        id: ObjectId,
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
                id,
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

    pub(crate) fn verify_payload_ref(
        envelope: &earmark_core::Envelope,
        payload: &StoredPayload,
    ) -> Result<(), StoreError> {
        let actual = payload.payload_ref();
        if actual != envelope.payload_ref {
            return Err(StoreError::PayloadIntegrityMismatch {
                object_id: envelope.id.as_str().to_string(),
                version_id: envelope.version_id.as_str().to_string(),
                expected: envelope.payload_ref.0.clone(),
                actual: actual.0,
            });
        }
        Ok(())
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

pub trait WorkspaceLayout {
    fn root(&self) -> &Path;
    fn init_layout(&self) -> Result<(), StoreError>;
    fn version_path(&self, version: &VersionRef) -> PathBuf;
}

pub trait ObjectStore {
    fn write_object(&self, object: &StoredObject) -> Result<VersionRef, StoreError>;
    fn write_batch(&self, batch: &BatchWrite) -> Result<Vec<VersionRef>, StoreError>;
    fn read_version(&self, version: &VersionRef) -> Result<StoredObject, StoreError>;
    fn read_head(&self, object_id: &ObjectId) -> Result<Option<StoredObject>, StoreError>;
    fn read_head_ref(&self, object_id: &ObjectId) -> Result<Option<VersionRef>, StoreError>;
    fn list_versions(&self, object_id: &ObjectId) -> Result<Vec<VersionRef>, StoreError>;
    fn resolve_payload(&self, payload_ref: &PayloadRef) -> Result<StoredPayload, StoreError>;
}

pub trait StoreScanner {
    fn scan_objects(&self) -> Result<Vec<StoredObject>, StoreError>;
}

pub trait StoreWriteLocking {
    fn acquire_write_lock(&self) -> Result<WorkspaceWriteGuard, StoreError>;
    fn write_batch_locked(
        &self,
        guard: &WorkspaceWriteGuard,
        batch: &BatchWrite,
    ) -> Result<Vec<VersionRef>, StoreError>;
    fn write_object_locked(
        &self,
        guard: &WorkspaceWriteGuard,
        object: &StoredObject,
    ) -> Result<VersionRef, StoreError> {
        self.write_batch_locked(
            guard,
            &BatchWrite {
                message: format!(
                    "deposit {} {}",
                    object.envelope.kind.as_str(),
                    object.envelope.version_id.as_str()
                ),
                objects: vec![object.clone()],
            },
        )?
        .into_iter()
        .next()
        .ok_or_else(|| StoreError::Invariant("batch write returned no refs".to_string()))
    }
}

pub trait CanonicalStore: WorkspaceLayout + ObjectStore + StoreScanner + StoreWriteLocking {}

#[derive(Debug, Clone)]
pub struct GitCanonicalStore {
    root: PathBuf,
    backend: GixBackend,
}

impl GitCanonicalStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            root: root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
            backend: GixBackend,
        }
    }

    pub fn canonical_dir(&self) -> PathBuf {
        self.root.join(".earmark").join("canonical")
    }

    pub(crate) fn objects_dir(&self) -> PathBuf {
        self.canonical_dir().join("objects")
    }

    pub(crate) fn payloads_dir(&self) -> PathBuf {
        self.canonical_dir().join("payloads")
    }

    pub(crate) fn heads_dir(&self) -> PathBuf {
        self.canonical_dir().join("heads")
    }

    pub(crate) fn derived_dir(&self) -> PathBuf {
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
        self.objects_dir()
            .join(object_id.as_str())
            .join(version_id.as_str())
    }

    fn head_path(&self, object_id: &ObjectId) -> PathBuf {
        self.heads_dir()
            .join(format!("{}.json", object_id.as_str()))
    }

    fn payload_path(&self, payload_ref: &PayloadRef, encoding: PayloadEncoding) -> PathBuf {
        let safe = payload_ref.0.replace(':', "_");
        self.payloads_dir()
            .join(format!("{}.{}", safe, encoding.extension()))
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

    pub fn layout_status(&self) -> WorkspaceLayoutStatus {
        WorkspaceLayoutStatus {
            root_exists: self.root.exists(),
            git_exists: self.canonical_dir().join(".git").exists(),
            manifest_exists: self.manifest_path().exists(),
            canonical_dir_exists: self.canonical_dir().exists(),
            objects_dir_exists: self.objects_dir().exists(),
            payloads_dir_exists: self.payloads_dir().exists(),
            heads_dir_exists: self.heads_dir().exists(),
            derived_dir_exists: self.derived_dir().exists(),
            work_surfaces_dir_exists: self.work_surfaces_dir().exists(),
            declarations_dir_exists: self.declarations_dir().exists(),
            corpus_dir_exists: self.corpus_dir().exists(),
        }
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join(".earmark_lock")
    }

    pub fn acquire_write_lock(&self) -> Result<WorkspaceWriteGuard, StoreError> {
        let lock_path = self.lock_path();
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = fs::File::create(&lock_path)?;
        file.lock_exclusive()?;
        Ok(WorkspaceWriteGuard { _file: file })
    }

    fn init_layout_unlocked(&self) -> Result<(), StoreError> {
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

        self.backend.ensure_repo(&self.canonical_dir())?;
        Ok(())
    }
}

pub struct WorkspaceWriteGuard {
    _file: fs::File,
}

impl WorkspaceLayout for GitCanonicalStore {
    fn root(&self) -> &Path {
        &self.root
    }

    fn init_layout(&self) -> Result<(), StoreError> {
        let _guard = self.acquire_write_lock()?;
        self.init_layout_unlocked()
    }

    fn version_path(&self, version: &VersionRef) -> PathBuf {
        self.version_dir(&version.id, &version.version_id)
    }
}

impl StoreWriteLocking for GitCanonicalStore {
    fn acquire_write_lock(&self) -> Result<WorkspaceWriteGuard, StoreError> {
        self.acquire_write_lock()
    }

    fn write_batch_locked(
        &self,
        _guard: &WorkspaceWriteGuard,
        batch: &BatchWrite,
    ) -> Result<Vec<VersionRef>, StoreError> {
        self.init_layout_unlocked()?;
        let mut written = Vec::with_capacity(batch.objects.len());
        let mut total_size = 0;
        let mut created_files: Vec<PathBuf> = Vec::new();
        let mut created_dirs: Vec<PathBuf> = Vec::new();
        let mut overwritten_heads: BTreeMap<PathBuf, Vec<u8>> = BTreeMap::new();

        for object in &batch.objects {
            earmark_core::validate_payload_size(object.payload.bytes.len())?;
            total_size += object.payload.bytes.len();
        }

        const MAX_BATCH_SIZE: usize = 32 * 1024 * 1024; // 32 MiB
        if total_size > MAX_BATCH_SIZE {
            return Err(StoreError::Core(earmark_core::CoreError::PayloadTooLarge(
                total_size,
                MAX_BATCH_SIZE,
            )));
        }

        let write_result: Result<(), StoreError> = (|| {
            for object in &batch.objects {
                if object.payload.payload_ref() != object.envelope.payload_ref {
                    return Err(StoreError::PayloadRefMismatch);
                }

                let payload_path =
                    self.payload_path(&object.envelope.payload_ref, object.payload.format);
                if !payload_path.exists() {
                    fs::write(&payload_path, &object.payload.bytes)?;
                    created_files.push(payload_path.clone());
                }

                let version_dir =
                    self.version_dir(&object.envelope.id, &object.envelope.version_id);
                if !version_dir.exists() {
                    created_dirs.push(version_dir.clone());
                }
                fs::create_dir_all(&version_dir)?;
                let envelope_path = version_dir.join("envelope.json");
                fs::write(&envelope_path, to_json_pretty(&object.envelope)?)?;
                created_files.push(envelope_path);
                let payload_version_path =
                    version_dir.join(format!("payload.{}", object.payload.format.extension()));
                fs::write(&payload_version_path, &object.payload.bytes)?;
                created_files.push(payload_version_path);
                let payload_ref_path = version_dir.join("payload_ref.txt");
                fs::write(&payload_ref_path, object.envelope.payload_ref.0.as_bytes())?;
                created_files.push(payload_ref_path);
                let head_path = self.head_path(&object.envelope.id);
                let head_existed = head_path.exists();
                if head_existed && !overwritten_heads.contains_key(&head_path) {
                    overwritten_heads.insert(head_path.clone(), fs::read(&head_path)?);
                }
                fs::write(&head_path, to_json_pretty(&object.envelope.version_ref())?)?;
                if !head_existed {
                    created_files.push(head_path);
                }
                written.push(object.envelope.version_ref());
            }
            self.backend
                .commit_paths(&self.canonical_dir(), &batch.message)?;
            Ok(())
        })();
        if let Err(err) = write_result {
            let mut rollback_failures = Vec::new();
            for file in created_files.iter().rev() {
                if file.exists() {
                    if let Err(e) = fs::remove_file(file) {
                        rollback_failures.push(format!("remove file {}: {}", file.display(), e));
                    }
                }
            }
            for dir in created_dirs.iter().rev() {
                if dir.exists() {
                    if let Err(e) = fs::remove_dir_all(dir) {
                        rollback_failures.push(format!("remove dir {}: {}", dir.display(), e));
                    }
                }
                // Try to clean up parent directories if they are empty
                let mut current = dir.parent();
                while let Some(path) = current {
                    if path == self.objects_dir()
                        || path == self.payloads_dir()
                        || path == self.heads_dir()
                        || path == self.canonical_dir()
                    {
                        break;
                    }
                    if path.exists() {
                        match fs::read_dir(path) {
                            Ok(mut entries) => {
                                if entries.next().is_none() {
                                    if let Err(e) = fs::remove_dir(path) {
                                        rollback_failures.push(format!(
                                            "remove parent dir {}: {}",
                                            path.display(),
                                            e
                                        ));
                                    }
                                } else {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    current = path.parent();
                }
            }
            for (head_path, previous_contents) in overwritten_heads.iter().rev() {
                if let Err(e) = fs::write(head_path, previous_contents) {
                    rollback_failures.push(format!("restore head {}: {}", head_path.display(), e));
                }
            }
            if rollback_failures.is_empty() {
                return Err(err);
            }
            return Err(StoreError::Rollback {
                write_error: err.to_string(),
                rollback_error: rollback_failures.join("; "),
            });
        }
        Ok(written)
    }
}

impl ObjectStore for GitCanonicalStore {
    fn write_object(&self, object: &StoredObject) -> Result<VersionRef, StoreError> {
        let guard = self.acquire_write_lock()?;
        self.write_object_locked(&guard, object)
    }

    fn write_batch(&self, batch: &BatchWrite) -> Result<Vec<VersionRef>, StoreError> {
        let guard = self.acquire_write_lock()?;
        self.write_batch_locked(&guard, batch)
    }

    fn read_version(&self, version: &VersionRef) -> Result<StoredObject, StoreError> {
        let version_dir = self.version_dir(&version.id, &version.version_id);
        let envelope_json = fs::read_to_string(version_dir.join("envelope.json"))?;
        let envelope: Envelope = serde_json::from_str(&envelope_json)?;

        let payload_path = fs::read_dir(&version_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.starts_with("payload."))
                    .unwrap_or(false)
            })
            .ok_or_else(|| StoreError::MissingPayload(version.version_id.as_str().to_string()))?;

        let format = Self::infer_encoding(&payload_path)?;
        let payload = StoredPayload::new(format, fs::read(payload_path)?);
        StoredObject::verify_payload_ref(&envelope, &payload)?;
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
        let object_dir = self.objects_dir().join(object_id.as_str());
        if !object_dir.exists() {
            return Ok(vec![]);
        }
        let mut refs = Vec::new();
        for entry in fs::read_dir(object_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let version_id = entry.file_name().to_string_lossy().to_string();
                refs.push(VersionRef::new(
                    object_id.clone(),
                    VersionId::parse(version_id)?,
                ));
            }
        }
        refs.sort_by(|a, b| a.version_id.as_str().cmp(b.version_id.as_str()));
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
}

impl StoreScanner for GitCanonicalStore {
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
                    .ok_or_else(|| {
                        StoreError::MissingPayload(envelope.version_id.as_str().to_string())
                    })?;
                let payload = StoredPayload::new(
                    Self::infer_encoding(&payload_path)?,
                    fs::read(payload_path)?,
                );
                StoredObject::verify_payload_ref(&envelope, &payload)?;
                objects.push(StoredObject { envelope, payload });
            }
        }
        Ok(objects)
    }
}

impl CanonicalStore for GitCanonicalStore {}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("git backend error: {0}")]
    GitBackend(String),
    #[error("git command failed (`{command}`): {stderr}")]
    GitCommandFailed { command: String, stderr: String },
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("utf8 error: {0}")]
    Utf8Slice(#[from] std::str::Utf8Error),
    #[error("core error: {0}")]
    Core(#[from] earmark_core::CoreError),
    #[error("payload ref mismatch between envelope and stored bytes")]
    PayloadRefMismatch,
    #[error("payload integrity mismatch for {object_id} version {version_id}: expected {expected}, actual {actual}")]
    PayloadIntegrityMismatch {
        object_id: String,
        version_id: String,
        expected: String,
        actual: String,
    },
    #[error("missing payload for {0}")]
    MissingPayload(String),
    #[error("unknown payload encoding: {0}")]
    UnknownPayloadEncoding(String),
    #[error("invariant violation: {0}")]
    Invariant(String),
    #[error("write failed: {write_error}; rollback failed: {rollback_error}")]
    Rollback {
        write_error: String,
        rollback_error: String,
    },
}
