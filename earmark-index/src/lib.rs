use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::Utc;
use earmark_core::{
    parse_json, parse_yaml, ClassDefinition, CompiledContextTemplate, DimensionId,
    InstructionPayload, Kind, ObjectId, ProviderProfile, RelationPayload, StandingPolicy,
    SystemDefinition, TokenId, UndoRecord, VersionRef, WorkflowDefinition,
};
use earmark_store::CanonicalStore;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSummary {
    pub object_id: String,
    pub version_id: String,
    pub kind: String,
    pub class: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub system_id: Option<String>,
    pub namespace: Option<String>,
    #[serde(default)]
    pub standing: BTreeMap<String, String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationEdge {
    pub version_id: String,
    pub relation_object_id: String,
    pub source_object_id: String,
    pub target_object_id: String,
    pub relation_type: String,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSystemRecord {
    pub namespace: String,
    pub system_id: String,
    pub object_id: String,
    pub version_id: String,
    pub activated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDirtyMarker {
    pub schema_version: String,
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub operation: String,
    pub object_ids: Vec<String>,
    pub version_ids: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct QueryFilter {
    pub class: Option<String>,
    pub kind: Option<String>,
    pub text: Option<String>,
    pub object_id: Option<String>,
    pub standing: BTreeMap<DimensionId, Vec<TokenId>>,
    pub include_undone: bool,
}

#[derive(Debug)]
pub struct DerivedIndex {
    conn: Connection,
    path: PathBuf,
}

impl DerivedIndex {
    fn index_path(root: impl AsRef<Path>) -> PathBuf {
        root.as_ref()
            .join(".earmark")
            .join("derived")
            .join("index.sqlite")
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self, IndexError> {
        let path = Self::index_path(root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        let index = Self { conn, path };
        index.init_schema()?;
        Ok(index)
    }

    pub fn open_existing(root: impl AsRef<Path>) -> Result<Self, IndexError> {
        let path = Self::index_path(root);
        if !path.exists() {
            return Err(IndexError::MissingIndex(path.display().to_string()));
        }
        let conn = Connection::open(&path)?;
        Ok(Self { conn, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn dirty_marker_path(&self) -> PathBuf {
        self.path.with_file_name("index_dirty.json")
    }

    pub fn mark_dirty(&self, marker: IndexDirtyMarker) -> Result<(), IndexError> {
        let path = self.dirty_marker_path();
        let json = serde_json::to_string_pretty(&marker)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn clear_dirty(&self) -> Result<(), IndexError> {
        let path = self.dirty_marker_path();
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn dirty_status(&self) -> Result<Option<IndexDirtyMarker>, IndexError> {
        let path = self.dirty_marker_path();
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(path)?;
        let marker = serde_json::from_str(&json)?;
        Ok(Some(marker))
    }

    pub fn is_run_undone(&self, run_id: &str) -> Result<Option<String>, IndexError> {
        let mut stmt = self
            .conn
            .prepare("SELECT undo_record_id FROM undo_records WHERE target_run_id = ?1")?;
        let res = stmt
            .query_row(params![run_id], |row| row.get::<_, String>(0))
            .optional()?;
        Ok(res)
    }

    fn init_schema(&self) -> Result<(), IndexError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS objects (
                version_id TEXT PRIMARY KEY,
                object_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                class TEXT,
                title TEXT,
                summary TEXT,
                payload_ref TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                system_id TEXT,
                namespace TEXT,
                declaration_identity TEXT,
                searchable_text TEXT,
                headers TEXT
            );

            CREATE TABLE IF NOT EXISTS heads (
                object_id TEXT PRIMARY KEY,
                version_id TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS relations (
                version_id TEXT PRIMARY KEY,
                relation_object_id TEXT NOT NULL,
                source_object_id TEXT NOT NULL,
                target_object_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                scope TEXT
            );

            CREATE TABLE IF NOT EXISTS active_systems (
                namespace TEXT PRIMARY KEY,
                system_id TEXT NOT NULL,
                object_id TEXT NOT NULL,
                version_id TEXT NOT NULL,
                activated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS active_assignment_claims (
                run_id TEXT NOT NULL,
                transition_id TEXT NOT NULL,
                assignment_id TEXT NOT NULL,
                claimed_at TEXT NOT NULL,
                PRIMARY KEY (run_id, transition_id)
            );

            CREATE TABLE IF NOT EXISTS object_standing (
                version_id TEXT NOT NULL,
                object_id TEXT NOT NULL,
                dimension TEXT NOT NULL,
                token TEXT NOT NULL,
                PRIMARY KEY (version_id, dimension)
            );

            CREATE TABLE IF NOT EXISTS undo_records (
                undo_record_id TEXT PRIMARY KEY,
                target_run_id TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS undone_objects (
                object_id TEXT PRIMARY KEY,
                undo_record_id TEXT NOT NULL,
                target_run_id TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS undone_relations (
                relation_object_id TEXT PRIMARY KEY,
                undo_record_id TEXT NOT NULL,
                target_run_id TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_object_standing_dimension_token
            ON object_standing(dimension, token);

            CREATE INDEX IF NOT EXISTS idx_object_standing_object
            ON object_standing(object_id);
            "#,
        )?;
        // Backfill for existing indexes created before declaration_identity existed.
        if let Err(err) = self.conn.execute(
            "ALTER TABLE objects ADD COLUMN declaration_identity TEXT",
            [],
        ) {
            match err {
                rusqlite::Error::SqliteFailure(_, Some(msg))
                    if msg.contains("duplicate column name") => {}
                _ => return Err(err.into()),
            }
        }
        if let Err(err) = self
            .conn
            .execute("ALTER TABLE objects ADD COLUMN headers TEXT", [])
        {
            match err {
                rusqlite::Error::SqliteFailure(_, Some(msg))
                    if msg.contains("duplicate column name") => {}
                _ => return Err(err.into()),
            }
        }
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_objects_kind_declaration_identity ON objects(kind, declaration_identity, updated_at)",
            [],
        )?;
        Ok(())
    }

    pub fn claim_active_assignment(
        &self,
        run_id: &str,
        transition_id: &str,
        assignment_id: &str,
    ) -> Result<(), IndexError> {
        let claimed_at = Utc::now().to_rfc3339();
        let updated = self.conn.execute(
            "INSERT INTO active_assignment_claims (run_id, transition_id, assignment_id, claimed_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(run_id, transition_id) DO NOTHING",
            params![run_id, transition_id, assignment_id, claimed_at],
        )?;
        if updated == 0 {
            return Err(IndexError::Conflict(format!(
                "active assignment already claimed for run {} transition {}",
                run_id, transition_id
            )));
        }
        Ok(())
    }

    pub fn release_active_assignment(
        &self,
        run_id: &str,
        transition_id: &str,
        assignment_id: &str,
    ) -> Result<(), IndexError> {
        self.conn.execute(
            "DELETE FROM active_assignment_claims WHERE run_id = ?1 AND transition_id = ?2 AND assignment_id = ?3",
            params![run_id, transition_id, assignment_id],
        )?;
        Ok(())
    }

    pub fn rebuild_from_store<S: CanonicalStore>(&self, store: &S) -> Result<(), IndexError> {
        store.init_layout()?;
        self.conn.execute("DELETE FROM objects", [])?;
        self.conn.execute("DELETE FROM heads", [])?;
        self.conn.execute("DELETE FROM relations", [])?;
        self.conn.execute("DELETE FROM object_standing", [])?;
        self.conn.execute("DELETE FROM undo_records", [])?;
        self.conn.execute("DELETE FROM undone_objects", [])?;
        self.conn.execute("DELETE FROM undone_relations", [])?;

        let diagnostics = store.scan_objects()?;
        let mut seen = std::collections::BTreeSet::new();
        for stored in diagnostics.scanned_objects {
            let envelope = stored.envelope;
            let payload = stored.payload;
            seen.insert(envelope.id.as_str().to_string());

            let title = envelope.title();
            let (summary, system_id, namespace, declaration_identity, searchable_text) =
                match &envelope.kind {
                    Kind::Instruction => {
                        let text = payload.as_utf8()?;
                        let parsed = InstructionPayload::parse_markdown(&text)?;
                        let declaration_name = parsed.name.clone();
                        (
                            Some(snippet(parsed.body.as_str())),
                            None,
                            None,
                            Some(declaration_name),
                            Some(format!(
                                "{} {} {}",
                                parsed.name,
                                parsed.purpose,
                                parsed.body.as_str()
                            )),
                        )
                    }
                    Kind::SystemDefinition => {
                        let text = payload.as_utf8()?;
                        let parsed: SystemDefinition = parse_yaml(&text)?;
                        (
                            parsed
                                .description
                                .clone()
                                .or_else(|| Some(parsed.title.clone())),
                            Some(parsed.system_id.clone()),
                            Some(parsed.namespace),
                            Some(parsed.system_id),
                            Some(text),
                        )
                    }
                    Kind::Relation => {
                        let text = payload.as_utf8()?;
                        let parsed: RelationPayload = parse_json(&text)?;
                        self.conn.execute(
                        "INSERT OR REPLACE INTO relations (version_id, relation_object_id, source_object_id, target_object_id, relation_type, scope) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            envelope.version_id.as_str().to_string(),
                            envelope.id.as_str().to_string(),
                            parsed.source.id.as_str().to_string(),
                            parsed.target.id.as_str().to_string(),
                            parsed.relation_type,
                            parsed.scope,
                        ],
                    )?;
                        (Some("relation".to_string()), None, None, None, Some(text))
                    }
                    Kind::CompiledContextTemplate => {
                        let text = payload.as_utf8()?;
                        let parsed: CompiledContextTemplate = parse_yaml(&text)?;
                        (
                            parsed.description.clone(),
                            None,
                            None,
                            Some(parsed.name),
                            Some(text),
                        )
                    }
                    Kind::Workflow => {
                        let text = payload.as_utf8()?;
                        let parsed: WorkflowDefinition = parse_yaml(&text)?;
                        (
                            parsed.description.clone(),
                            None,
                            None,
                            Some(parsed.name),
                            Some(text),
                        )
                    }
                    Kind::Policy => {
                        let text = payload.as_utf8()?;
                        let parsed: StandingPolicy = parse_yaml(&text)?;
                        (
                            parsed.description.clone(),
                            None,
                            None,
                            Some(parsed.name),
                            Some(text),
                        )
                    }
                    Kind::ProviderProfile => {
                        let text = payload.as_utf8()?;
                        let parsed: ProviderProfile = parse_yaml(&text)?;
                        (
                            parsed.description.clone(),
                            None,
                            None,
                            Some(parsed.name),
                            Some(text),
                        )
                    }
                    Kind::Object if envelope.class.as_deref() == Some("class_definition") => {
                        let text = payload.as_utf8()?;
                        let parsed: ClassDefinition = parse_yaml(&text)?;
                        (
                            Some(snippet(&text)),
                            None,
                            None,
                            Some(parsed.name),
                            Some(text),
                        )
                    }
                    Kind::UndoRecord => {
                        let text = payload.as_utf8()?;
                        let parsed: UndoRecord = parse_json(&text)?;
                        self.conn.execute(
                            "INSERT OR REPLACE INTO undo_records (undo_record_id, target_run_id, created_at) VALUES (?1, ?2, ?3)",
                            params![
                                envelope.id.as_str().to_string(),
                                parsed.target_run_id.clone(),
                                envelope.created_at.to_rfc3339(),
                            ],
                        )?;
                        for oid in &parsed.created_object_ids {
                            self.conn.execute(
                                "INSERT OR REPLACE INTO undone_objects (object_id, undo_record_id, target_run_id) VALUES (?1, ?2, ?3)",
                                params![
                                    oid.as_str().to_string(),
                                    envelope.id.as_str().to_string(),
                                    parsed.target_run_id.clone(),
                                ],
                            )?;
                        }
                        for rid in &parsed.created_relation_ids {
                            self.conn.execute(
                                "INSERT OR REPLACE INTO undone_relations (relation_object_id, undo_record_id, target_run_id) VALUES (?1, ?2, ?3)",
                                params![
                                    rid.as_str().to_string(),
                                    envelope.id.as_str().to_string(),
                                    parsed.target_run_id.clone(),
                                ],
                            )?;
                        }
                        (
                            Some("undo record".to_string()),
                            None,
                            None,
                            None,
                            Some(text),
                        )
                    }
                    _ => {
                        let text = payload.as_utf8().unwrap_or_default();
                        (Some(snippet(&text)), None, None, None, Some(text))
                    }
                };

            let version_id = envelope.version_id.as_str().to_string();
            let object_id = envelope.id.as_str().to_string();
            let kind = envelope.kind.as_str().to_string();
            let class = envelope.class.clone();
            let payload_ref = envelope.payload_ref.0.clone();
            let created_at = envelope.created_at.to_rfc3339();
            let updated_at = envelope.updated_at.to_rfc3339();

            self.conn.execute(
                "INSERT OR REPLACE INTO objects (
                    version_id, object_id, kind, class, title, summary,
                    payload_ref, created_at, updated_at, system_id, namespace, declaration_identity, searchable_text, headers
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    version_id,
                    object_id,
                    kind,
                    class,
                    title,
                    summary,
                    payload_ref,
                    created_at,
                    updated_at,
                    system_id,
                    namespace,
                    declaration_identity,
                    searchable_text,
                    serde_json::to_string(&envelope.headers).unwrap_or_default(),
                ],
            )?;

            for (dim, token) in envelope.standing.iter() {
                self.conn.execute(
                    "INSERT OR REPLACE INTO object_standing (version_id, object_id, dimension, token) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        envelope.version_id.as_str(),
                        envelope.id.as_str(),
                        dim.as_str(),
                        token.as_str(),
                    ],
                )?;
            }
        }

        for object_id in seen {
            let object_id = ObjectId::parse(object_id)?;
            if let Some(head) = store.read_head_ref(&object_id)? {
                self.conn.execute(
                    "INSERT OR REPLACE INTO heads (object_id, version_id) VALUES (?1, ?2)",
                    params![head.id.as_str(), head.version_id.as_str()],
                )?;
            }
        }
        Ok(())
    }

    pub fn upsert_head_object_from_store<S: CanonicalStore>(
        &self,
        store: &S,
        object_id: &ObjectId,
    ) -> Result<(), IndexError> {
        let Some(head) = store.read_head(object_id)? else {
            self.conn.execute(
                "DELETE FROM heads WHERE object_id = ?1",
                params![object_id.as_str()],
            )?;
            return Ok(());
        };

        let envelope = head.envelope.clone();
        let payload = head.payload.clone();
        let title = envelope.title();
        let (summary, system_id, namespace, declaration_identity, searchable_text) = match &envelope
            .kind
        {
            Kind::Instruction => {
                let text = payload.as_utf8()?;
                let parsed = InstructionPayload::parse_markdown(&text)?;
                let declaration_name = parsed.name.clone();
                (
                    Some(snippet(parsed.body.as_str())),
                    None,
                    None,
                    Some(declaration_name),
                    Some(format!(
                        "{} {} {}",
                        parsed.name,
                        parsed.purpose,
                        parsed.body.as_str()
                    )),
                )
            }
            Kind::SystemDefinition => {
                let text = payload.as_utf8()?;
                let parsed: SystemDefinition = parse_yaml(&text)?;
                (
                    parsed
                        .description
                        .clone()
                        .or_else(|| Some(parsed.title.clone())),
                    Some(parsed.system_id.clone()),
                    Some(parsed.namespace),
                    Some(parsed.system_id),
                    Some(text),
                )
            }
            Kind::Relation => {
                let text = payload.as_utf8()?;
                let parsed: RelationPayload = parse_json(&text)?;
                self.conn.execute(
                        "INSERT OR REPLACE INTO relations (version_id, relation_object_id, source_object_id, target_object_id, relation_type, scope) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            envelope.version_id.as_str().to_string(),
                            envelope.id.as_str().to_string(),
                            parsed.source.id.as_str().to_string(),
                            parsed.target.id.as_str().to_string(),
                            parsed.relation_type,
                            parsed.scope,
                        ],
                    )?;
                (Some("relation".to_string()), None, None, None, Some(text))
            }
            Kind::CompiledContextTemplate => {
                let text = payload.as_utf8()?;
                let parsed: CompiledContextTemplate = parse_yaml(&text)?;
                (
                    parsed.description.clone(),
                    None,
                    None,
                    Some(parsed.name),
                    Some(text),
                )
            }
            Kind::Workflow => {
                let text = payload.as_utf8()?;
                let parsed: WorkflowDefinition = parse_yaml(&text)?;
                (
                    parsed.description.clone(),
                    None,
                    None,
                    Some(parsed.name),
                    Some(text),
                )
            }
            Kind::Policy => {
                let text = payload.as_utf8()?;
                let parsed: StandingPolicy = parse_yaml(&text)?;
                (
                    parsed.description.clone(),
                    None,
                    None,
                    Some(parsed.name),
                    Some(text),
                )
            }
            Kind::ProviderProfile => {
                let text = payload.as_utf8()?;
                let parsed: ProviderProfile = parse_yaml(&text)?;
                (
                    parsed.description.clone(),
                    None,
                    None,
                    Some(parsed.name),
                    Some(text),
                )
            }
            Kind::Object if envelope.class.as_deref() == Some("class_definition") => {
                let text = payload.as_utf8()?;
                let parsed: ClassDefinition = parse_yaml(&text)?;
                (
                    Some(snippet(&text)),
                    None,
                    None,
                    Some(parsed.name),
                    Some(text),
                )
            }
            Kind::UndoRecord => {
                let text = payload.as_utf8()?;
                let parsed: UndoRecord = parse_json(&text)?;
                self.conn.execute(
                    "INSERT OR REPLACE INTO undo_records (undo_record_id, target_run_id, created_at) VALUES (?1, ?2, ?3)",
                    params![
                        envelope.id.as_str().to_string(),
                        parsed.target_run_id.clone(),
                        envelope.created_at.to_rfc3339(),
                    ],
                )?;
                for oid in &parsed.created_object_ids {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO undone_objects (object_id, undo_record_id, target_run_id) VALUES (?1, ?2, ?3)",
                        params![
                            oid.as_str().to_string(),
                            envelope.id.as_str().to_string(),
                            parsed.target_run_id.clone(),
                        ],
                    )?;
                }
                for rid in &parsed.created_relation_ids {
                    self.conn.execute(
                        "INSERT OR REPLACE INTO undone_relations (relation_object_id, undo_record_id, target_run_id) VALUES (?1, ?2, ?3)",
                        params![
                            rid.as_str().to_string(),
                            envelope.id.as_str().to_string(),
                            parsed.target_run_id.clone(),
                        ],
                    )?;
                }
                (
                    Some("undo record".to_string()),
                    None,
                    None,
                    None,
                    Some(text),
                )
            }
            _ => {
                let text = payload.as_utf8().unwrap_or_default();
                (Some(snippet(&text)), None, None, None, Some(text))
            }
        };

        self.conn.execute(
            "INSERT OR REPLACE INTO objects (
                version_id, object_id, kind, class, title, summary,
                payload_ref, created_at, updated_at, system_id, namespace, declaration_identity, searchable_text, headers
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                envelope.version_id.as_str().to_string(),
                envelope.id.as_str().to_string(),
                envelope.kind.as_str().to_string(),
                envelope.class.clone(),
                title,
                summary,
                envelope.payload_ref.0.clone(),
                envelope.created_at.to_rfc3339(),
                envelope.updated_at.to_rfc3339(),
                system_id,
                namespace,
                declaration_identity,
                searchable_text,
                serde_json::to_string(&envelope.headers).unwrap_or_default(),
            ],
        )?;
        self.conn.execute(
            "INSERT OR REPLACE INTO heads (object_id, version_id) VALUES (?1, ?2)",
            params![envelope.id.as_str(), envelope.version_id.as_str()],
        )?;

        let version_id_str = envelope.version_id.as_str();
        self.conn.execute(
            "DELETE FROM object_standing WHERE version_id = ?1",
            params![version_id_str],
        )?;
        for (dim, token) in envelope.standing.iter() {
            self.conn.execute(
                "INSERT INTO object_standing (version_id, object_id, dimension, token) VALUES (?1, ?2, ?3, ?4)",
                params![
                    version_id_str,
                    envelope.id.as_str(),
                    dim.as_str(),
                    token.as_str(),
                ],
            )?;
        }
        Ok(())
    }

    pub fn query_objects(&self, filter: &QueryFilter) -> Result<Vec<ObjectSummary>, IndexError> {
        let mut sql = String::from(
            "SELECT o.object_id, o.version_id, o.kind, o.class, o.title, o.summary, o.system_id, o.namespace, o.headers FROM objects o JOIN heads h ON o.object_id = h.object_id AND o.version_id = h.version_id WHERE 1=1",
        );
        let mut values: Vec<String> = Vec::new();

        if let Some(class) = &filter.class {
            sql.push_str(" AND o.class = ?");
            values.push(class.clone());
        }
        if let Some(kind) = &filter.kind {
            sql.push_str(" AND o.kind = ?");
            values.push(kind.clone());
        }
        if let Some(text) = &filter.text {
            sql.push_str(" AND COALESCE(o.searchable_text, '') LIKE ?");
            values.push(format!("%{}%", text));
        }
        if let Some(object_id) = &filter.object_id {
            sql.push_str(" AND o.object_id = ?");
            values.push(object_id.clone());
        }

        if !filter.include_undone {
            sql.push_str(
                " AND NOT EXISTS (SELECT 1 FROM undone_objects u WHERE u.object_id = o.object_id)",
            );
        }

        for (dim, tokens) in &filter.standing {
            if tokens.is_empty() {
                continue;
            }
            let placeholders: Vec<String> = (0..tokens.len()).map(|_| "?".to_string()).collect();
            sql.push_str(&format!(
                " AND EXISTS (SELECT 1 FROM object_standing os WHERE os.version_id = o.version_id AND os.dimension = ? AND os.token IN ({}))",
                placeholders.join(",")
            ));
            values.push(dim.as_str().to_string());
            for token in tokens {
                values.push(token.as_str().to_string());
            }
        }

        sql.push_str(" ORDER BY o.updated_at DESC");

        let results: Vec<ObjectSummary> = {
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(rusqlite::params_from_iter(values.iter()), |row| {
                Ok(ObjectSummary {
                    object_id: row.get(0)?,
                    version_id: row.get(1)?,
                    kind: row.get(2)?,
                    class: row.get(3)?,
                    title: row.get(4)?,
                    summary: row.get(5)?,
                    system_id: row.get(6)?,
                    namespace: row.get(7)?,
                    standing: BTreeMap::new(),
                    headers: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let mut results = results;
        if !results.is_empty() {
            let version_ids: Vec<String> = results.iter().map(|r| r.version_id.clone()).collect();
            let placeholders: Vec<String> =
                (0..version_ids.len()).map(|_| "?".to_string()).collect();
            let mut stmt = self.conn.prepare(&format!(
                "SELECT version_id, dimension, token FROM object_standing WHERE version_id IN ({})",
                placeholders.join(",")
            ))?;
            let standing_rows =
                stmt.query_map(rusqlite::params_from_iter(version_ids.iter()), |row| {
                    let vid: String = row.get(0)?;
                    let dim: String = row.get(1)?;
                    let token: String = row.get(2)?;
                    Ok((vid, dim, token))
                })?;
            let mut acc: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
            for row in standing_rows {
                let (vid, dim, token) = row?;
                acc.entry(vid).or_default().insert(dim, token);
            }
            for result in &mut results {
                if let Some(map) = acc.remove(&result.version_id) {
                    result.standing = map;
                }
            }
        }

        Ok(results)
    }

    pub fn relation_adjacency(
        &self,
        object_id: &ObjectId,
        include_undone: bool,
    ) -> Result<Vec<RelationEdge>, IndexError> {
        let mut sql = String::from("SELECT version_id, relation_object_id, source_object_id, target_object_id, relation_type, scope FROM relations WHERE (source_object_id = ?1 OR target_object_id = ?1)");

        if !include_undone {
            sql.push_str(" AND NOT EXISTS (SELECT 1 FROM undone_relations ur WHERE ur.relation_object_id = relations.relation_object_id)");
        }

        sql.push_str(" ORDER BY version_id ASC");

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![object_id.as_str()], |row| {
            Ok(RelationEdge {
                version_id: row.get(0)?,
                relation_object_id: row.get(1)?,
                source_object_id: row.get(2)?,
                target_object_id: row.get(3)?,
                relation_type: row.get(4)?,
                scope: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn activate_system(
        &self,
        namespace: &str,
        system_id: &str,
        version_ref: &VersionRef,
    ) -> Result<ActiveSystemRecord, IndexError> {
        let activated_at = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO active_systems (namespace, system_id, object_id, version_id, activated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![namespace, system_id, version_ref.id.as_str(), version_ref.version_id.as_str(), activated_at.clone()],
        )?;
        Ok(ActiveSystemRecord {
            namespace: namespace.to_string(),
            system_id: system_id.to_string(),
            object_id: version_ref.id.as_str().to_string(),
            version_id: version_ref.version_id.as_str().to_string(),
            activated_at,
        })
    }

    pub fn get_active_system(
        &self,
        namespace: &str,
    ) -> Result<Option<ActiveSystemRecord>, IndexError> {
        self.conn
            .query_row(
                "SELECT namespace, system_id, object_id, version_id, activated_at FROM active_systems WHERE namespace = ?1",
                params![namespace],
                |row| {
                    Ok(ActiveSystemRecord {
                        namespace: row.get(0)?,
                        system_id: row.get(1)?,
                        object_id: row.get(2)?,
                        version_id: row.get(3)?,
                        activated_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(IndexError::from)
    }

    pub fn find_class_definition(
        &self,
        name: &str,
    ) -> Result<Option<(String, String)>, IndexError> {
        self.conn
            .query_row(
                "SELECT object_id, version_id FROM objects WHERE kind = 'object' AND class = 'class_definition' AND declaration_identity = ?1 ORDER BY updated_at DESC LIMIT 1",
                params![name],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(IndexError::from)
    }

    pub fn find_system_definition(
        &self,
        system_id: &str,
    ) -> Result<Option<(String, String, String)>, IndexError> {
        self.conn
            .query_row(
                "SELECT object_id, version_id, namespace FROM objects WHERE kind = ?1 AND system_id = ?2 ORDER BY updated_at DESC LIMIT 1",
                params![Kind::SystemDefinition.as_str(), system_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(IndexError::from)
    }

    pub fn find_latest_by_symbolic_name(
        &self,
        kind: &str,
        name: &str,
    ) -> Result<Option<(String, String)>, IndexError> {
        self.conn
            .query_row(
                "SELECT object_id, version_id FROM objects WHERE kind = ?1 AND declaration_identity = ?2 ORDER BY updated_at DESC LIMIT 1",
                params![kind, name],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(IndexError::from)
    }

    fn resolve_symbolic_latest_ref(
        &self,
        kind: &str,
        name: &str,
    ) -> Result<Option<VersionRef>, IndexError> {
        let Some((object_id, version_id)) = self.find_latest_by_symbolic_name(kind, name)? else {
            return Ok(None);
        };
        Ok(Some(VersionRef::new(
            ObjectId::parse(object_id)?,
            earmark_core::VersionId::parse(version_id)?,
        )))
    }

    pub fn resolve_workflow_symbolic_latest(
        &self,
        name: &str,
    ) -> Result<Option<VersionRef>, IndexError> {
        self.resolve_symbolic_latest_ref(Kind::Workflow.as_str(), name)
    }

    pub fn resolve_instruction_symbolic_latest(
        &self,
        name: &str,
    ) -> Result<Option<VersionRef>, IndexError> {
        self.resolve_symbolic_latest_ref(Kind::Instruction.as_str(), name)
    }

    pub fn resolve_class_definition_symbolic_latest(
        &self,
        name: &str,
    ) -> Result<Option<VersionRef>, IndexError> {
        let Some((object_id, version_id)) = self.find_class_definition(name)? else {
            return Ok(None);
        };
        Ok(Some(VersionRef::new(
            ObjectId::parse(object_id)?,
            earmark_core::VersionId::parse(version_id)?,
        )))
    }

    pub fn resolve_compiled_context_symbolic_latest(
        &self,
        name: &str,
    ) -> Result<Option<VersionRef>, IndexError> {
        self.resolve_symbolic_latest_ref(Kind::CompiledContextTemplate.as_str(), name)
    }

    pub fn resolve_provider_profile_symbolic_latest(
        &self,
        name: &str,
    ) -> Result<Option<VersionRef>, IndexError> {
        self.resolve_symbolic_latest_ref(Kind::ProviderProfile.as_str(), name)
    }

    pub fn resolve_standing_policy_symbolic_latest(
        &self,
        name: &str,
    ) -> Result<Option<VersionRef>, IndexError> {
        self.resolve_symbolic_latest_ref(Kind::Policy.as_str(), name)
    }

    pub fn resolve_system_definition_symbolic_latest(
        &self,
        name: &str,
    ) -> Result<Option<VersionRef>, IndexError> {
        let Some((object_id, version_id, _namespace)) = self.find_system_definition(name)? else {
            return Ok(None);
        };
        Ok(Some(VersionRef::new(
            ObjectId::parse(object_id)?,
            earmark_core::VersionId::parse(version_id)?,
        )))
    }

    pub fn get_active_systems(&self) -> Result<Vec<ActiveSystemRecord>, IndexError> {
        let mut stmt = self.conn.prepare(
            "SELECT namespace, system_id, object_id, version_id, activated_at FROM active_systems",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ActiveSystemRecord {
                namespace: row.get(0)?,
                system_id: row.get(1)?,
                object_id: row.get(2)?,
                version_id: row.get(3)?,
                activated_at: row.get(4)?,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn counts(&self) -> Result<(u64, u64), IndexError> {
        let objects: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM heads", [], |row| row.get(0))?;
        let active_systems: u64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM active_systems", [], |row| row.get(0))?;
        Ok((objects, active_systems))
    }

    pub fn object_count(&self) -> Result<u64, IndexError> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM objects", [], |row| row.get(0))?)
    }

    pub fn head_count(&self) -> Result<u64, IndexError> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM heads", [], |row| row.get(0))?)
    }

    pub fn relation_count(&self) -> Result<u64, IndexError> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))?)
    }

    pub fn get_objects_by_kind(&self, kind: Kind) -> Result<Vec<VersionRef>, IndexError> {
        let mut stmt = self.conn.prepare(
            "SELECT o.object_id, o.version_id FROM objects o JOIN heads h ON o.object_id = h.object_id AND o.version_id = h.version_id WHERE o.kind = ?1",
        )?;
        let rows = stmt.query_map(params![kind.as_str()], |row| {
            let object_id_str: String = row.get(0)?;
            let version_id_str: String = row.get(1)?;
            Ok((object_id_str, version_id_str))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (oid, vid) = row?;
            results.push(VersionRef::new(
                ObjectId::parse(oid).map_err(IndexError::Core)?,
                earmark_core::VersionId::parse(vid).map_err(IndexError::Core)?,
            ));
        }
        Ok(results)
    }

    pub fn get_head(&self, object_id: &ObjectId) -> Result<Option<VersionRef>, IndexError> {
        self.conn
            .query_row(
                "SELECT version_id FROM heads WHERE object_id = ?1",
                params![object_id.as_str()],
                |row| {
                    let vid: String = row.get(0)?;
                    Ok(VersionRef::new(
                        object_id.clone(),
                        earmark_core::VersionId::parse(vid).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?,
                    ))
                },
            )
            .optional()
            .map_err(IndexError::from)
    }
}

fn snippet(input: &str) -> String {
    let stripped = input
        .lines()
        .filter(|line| !line.trim_start().starts_with("---"))
        .collect::<Vec<_>>()
        .join(" ");
    stripped.chars().take(240).collect()
}

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("sql error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("store error: {0}")]
    Store(#[from] earmark_store::StoreError),
    #[error("core error: {0}")]
    Core(#[from] earmark_core::CoreError),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("missing index: {0}")]
    MissingIndex(String),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}
