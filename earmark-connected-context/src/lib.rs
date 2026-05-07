use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use chrono::Utc;
use earmark_core::{
    parse_yaml, CompiledContextTemplate, ObjectId, ObjectRef, ScalarValue, VersionRef,
};
use earmark_index::{DerivedIndex, ObjectSummary, QueryFilter};
use earmark_store::CanonicalStore;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSurfaceManifest {
    pub surface_id: String,
    pub compiled_context: VersionRef,
    pub work_packet: Option<ObjectRef>,
    pub generated_at: earmark_core::Timestamp,
    pub objects: Vec<WorkSurfaceObject>,
    pub constraints: BTreeMap<String, ScalarValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSurfaceObject {
    pub object: ObjectRef,
    pub title: Option<String>,
    pub path: String,
    pub excerpt_range: Option<String>,
    pub lineage: Vec<ObjectRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledContextPlan {
    pub template: VersionRef,
    pub selected_object_ids: Vec<String>,
    pub relation_expansion_count: usize,
    pub grouped_by: Vec<String>,
}

pub struct CompiledContextService;

impl CompiledContextService {
    pub fn plan<S: CanonicalStore>(
        store: &S,
        index: &DerivedIndex,
        template_ref: &VersionRef,
    ) -> Result<CompiledContextPlan, ProjectError> {
        let template = load_compiled_context_template(store, template_ref)?;
        let selected = collect_selected_objects(store, index, &template)?;
        let mut ids = selected
            .iter()
            .map(|s| s.object_id.clone())
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        Ok(CompiledContextPlan {
            template: template_ref.clone(),
            selected_object_ids: ids,
            relation_expansion_count: template.select.relations.len(),
            grouped_by: template.group_by,
        })
    }

    pub fn compile<S: CanonicalStore>(
        store: &S,
        index: &DerivedIndex,
        template_ref: &VersionRef,
        work_packet: Option<ObjectRef>,
    ) -> Result<WorkSurfaceManifest, ProjectError> {
        let template = load_compiled_context_template(store, template_ref)?;
        let selected = collect_selected_objects(store, index, &template)?;

        let objects = selected
            .into_iter()
            .map(|summary| {
                let version = VersionRef::new(
                    earmark_core::ObjectId(summary.object_id.clone()),
                    earmark_core::VersionId(summary.version_id.clone()),
                );
                let loaded = store.read_version(&version)?;
                Ok(WorkSurfaceObject {
                    object: loaded.envelope.object_ref(),
                    title: loaded.envelope.title(),
                    path: store.version_path(&version).display().to_string(),
                    excerpt_range: None,
                    lineage: loaded
                        .envelope
                        .provenance
                        .lineage
                        .iter()
                        .map(|link| link.object.clone())
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, ProjectError>>()?;

        let surface_id = format!(
            "ws_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let surface_dir = store
            .root()
            .join(".earmark")
            .join("work_surfaces")
            .join(&surface_id);
        fs::create_dir_all(&surface_dir)?;

        let constraints = BTreeMap::from([
            (
                "render_mode".to_string(),
                ScalarValue::String(template.render.mode.clone()),
            ),
            (
                "group_by_count".to_string(),
                ScalarValue::Integer(template.group_by.len() as i64),
            ),
            (
                "include_lineage".to_string(),
                ScalarValue::Bool(template.visibility.include_lineage),
            ),
            (
                "include_constraints".to_string(),
                ScalarValue::Bool(template.visibility.include_constraints),
            ),
            (
                "include_provenance".to_string(),
                ScalarValue::Bool(template.visibility.include_provenance),
            ),
        ]);

        let manifest = WorkSurfaceManifest {
            surface_id: surface_id.clone(),
            compiled_context: template_ref.clone(),
            work_packet,
            generated_at: Utc::now(),
            objects,
            constraints,
        };

        fs::write(
            surface_dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest)?,
        )?;
        fs::write(surface_dir.join("README.md"), render_readme(&manifest))?;
        fs::write(
            surface_dir.join("evidence.md"),
            render_evidence_pack(&manifest),
        )?;

        Ok(manifest)
    }

    pub fn cli_summary(manifest: &WorkSurfaceManifest) -> String {
        format!(
            "surface {} with {} object(s)",
            manifest.surface_id,
            manifest.objects.len()
        )
    }
}

fn load_compiled_context_template<S: CanonicalStore>(
    store: &S,
    template_ref: &VersionRef,
) -> Result<CompiledContextTemplate, ProjectError> {
    let template_object = store.read_version(template_ref)?;
    let template_text = template_object.payload.as_utf8()?;
    Ok(parse_yaml(&template_text)?)
}

fn collect_selected_objects<S: CanonicalStore>(
    _store: &S,
    index: &DerivedIndex,
    template: &CompiledContextTemplate,
) -> Result<Vec<ObjectSummary>, ProjectError> {
    let mut selected = Vec::new();
    let mut seen = BTreeSet::new();

    for class in &template.select.classes {
        let rows = index.query_objects(&QueryFilter {
            class: Some(class.clone()),
            ..Default::default()
        })?;
        for row in rows {
            if matches_standing(&row, &template.select.standing)
                && seen.insert((row.object_id.clone(), row.version_id.clone()))
            {
                selected.push(row);
            }
        }
    }

    if !template.select.relations.is_empty() {
        let mut additions = Vec::new();
        let mut added_ids = BTreeSet::new();
        for row in &selected {
            let edges = index.relation_adjacency(&ObjectId(row.object_id.clone()))?;
            for edge in edges
                .into_iter()
                .filter(|edge| template.select.relations.contains(&edge.relation_type))
            {
                let related_id = if edge.source_object_id == row.object_id {
                    edge.target_object_id
                } else {
                    edge.source_object_id
                };
                if added_ids.insert(related_id.clone()) {
                    let related_rows = index.query_objects(&QueryFilter {
                        object_id: Some(related_id),
                        ..Default::default()
                    })?;
                    for related in related_rows {
                        if seen.insert((related.object_id.clone(), related.version_id.clone())) {
                            additions.push(related);
                        }
                    }
                }
            }
        }
        selected.extend(additions);
    }

    selected.sort_by(|a, b| {
        a.title
            .clone()
            .unwrap_or_default()
            .cmp(&b.title.clone().unwrap_or_default())
            .then(a.object_id.cmp(&b.object_id))
    });
    Ok(selected)
}

fn matches_standing(row: &ObjectSummary, standing_filters: &BTreeMap<String, Vec<String>>) -> bool {
    standing_filters.iter().all(|(dimension, allowed)| {
        if allowed.is_empty() {
            return true;
        }
        let current = match dimension.as_str() {
            "epistemic" => &row.standing_epistemic,
            "review" => &row.standing_review,
            "process" => &row.standing_process,
            _ => return true,
        };
        allowed.iter().any(|candidate| candidate == current)
    })
}

fn render_readme(manifest: &WorkSurfaceManifest) -> String {
    let mut text = format!("# Work Surface {}\n\n", manifest.surface_id);
    for item in &manifest.objects {
        text.push_str(&format!(
            "- {} ({})\n",
            item.title.clone().unwrap_or_else(|| "untitled".to_string()),
            item.object.id.0
        ));
    }
    text
}

fn render_evidence_pack(manifest: &WorkSurfaceManifest) -> String {
    let mut text = format!("# Evidence Pack {}\n\n", manifest.surface_id);
    for item in &manifest.objects {
        text.push_str(&format!(
            "## {}\n\n- Object: {}\n- Path: {}\n- Version: {}\n\n",
            item.title.clone().unwrap_or_else(|| "untitled".to_string()),
            item.object.id.0,
            item.path,
            item.object.version_id.0,
        ));
    }
    text
}

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("store error: {0}")]
    Store(#[from] earmark_store::StoreError),
    #[error("index error: {0}")]
    Index(#[from] IndexError),
    #[error("core error: {0}")]
    Core(#[from] earmark_core::CoreError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

use earmark_index::IndexError;

#[cfg(test)]
mod tests {
    use super::*;
    use earmark_core::Kind;
    use earmark_index::ObjectSummary;

    #[test]
    fn test_matches_standing() {
        let row = ObjectSummary {
            object_id: "o1".to_string(),
            version_id: "v1".to_string(),
            kind: "Object".to_string(),
            class: Some("c1".to_string()),
            title: None,
            summary: None,
            standing_review: "accepted".to_string(),
            standing_process: "active".to_string(),
            standing_epistemic: "confirmed".to_string(),
            system_id: None,
            namespace: None,
        };

        let mut filters = BTreeMap::new();
        filters.insert("review".to_string(), vec!["accepted".to_string()]);
        assert!(matches_standing(&row, &filters));

        filters.insert("review".to_string(), vec!["rejected".to_string()]);
        assert!(!matches_standing(&row, &filters));

        let empty_filters = BTreeMap::new();
        assert!(matches_standing(&row, &empty_filters));
    }

    #[test]
    fn test_cli_summary() {
        let manifest = WorkSurfaceManifest {
            surface_id: "test".to_string(),
            compiled_context: VersionRef::new(
                ObjectId("o".to_string()),
                earmark_core::VersionId("v".to_string()),
            ),
            work_packet: None,
            generated_at: Utc::now(),
            objects: vec![WorkSurfaceObject {
                object: ObjectRef {
                    id: ObjectId("o1".to_string()),
                    version_id: earmark_core::VersionId("v1".to_string()),
                    kind: Kind::Object,
                    class: None,
                },
                title: None,
                path: "p".to_string(),
                excerpt_range: None,
                lineage: vec![],
            }],
            constraints: BTreeMap::new(),
        };
        assert_eq!(
            CompiledContextService::cli_summary(&manifest),
            "surface test with 1 object(s)"
        );
    }
}
