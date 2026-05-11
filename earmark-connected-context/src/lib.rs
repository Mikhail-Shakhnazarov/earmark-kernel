use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
};

mod filter;

use crate::filter::{object_summary_admissible, relation_type_admissible};
use chrono::Utc;
use earmark_core::{
    parse_yaml, CompiledContextTemplate, ExpansionObjectFilter, ObjectId, ObjectRef, ScalarValue,
    VersionRef,
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
    #[serde(default)]
    pub boundary_relations: Vec<WorkSurfaceBoundaryRelation>,
    pub constraints: BTreeMap<String, ScalarValue>,
    #[serde(default)]
    pub warnings: Vec<String>,
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
pub struct WorkSurfaceBoundaryRelation {
    pub relation: ObjectRef,
    pub relation_type: String,
    pub source: ObjectRef,
    pub target: ObjectRef,
    pub included_endpoint: ObjectRef,
    pub excluded_endpoint: ObjectRef,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledContextPlan {
    pub template: VersionRef,
    pub selected_object_ids: Vec<String>,
    pub relation_expansion_count: usize,
    pub boundary_relation_count: usize,
    pub grouped_by: Vec<String>,
    pub warnings: Vec<String>,
}

struct CompiledContextSelection {
    objects: Vec<ObjectSummary>,
    boundary_relations: Vec<BoundaryRelationCandidate>,
    warnings: Vec<String>,
}

struct BoundaryRelationCandidate {
    relation_object_id: String,
    relation_version_id: String,
    relation_type: String,
    included_object_id: String,
    excluded_object_id: String,
}

pub struct CompiledContextService;

pub trait CompiledContextCompiler<S: CanonicalStore> {
    fn compile(
        &self,
        store: &S,
        index: &DerivedIndex,
        template_ref: &VersionRef,
        work_packet: Option<ObjectRef>,
    ) -> Result<WorkSurfaceManifest, ProjectError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CanonicalCompiledContextCompiler;

impl<S: CanonicalStore> CompiledContextCompiler<S> for CanonicalCompiledContextCompiler {
    fn compile(
        &self,
        store: &S,
        index: &DerivedIndex,
        template_ref: &VersionRef,
        work_packet: Option<ObjectRef>,
    ) -> Result<WorkSurfaceManifest, ProjectError> {
        CompiledContextService::compile(store, index, template_ref, work_packet)
    }
}

pub const DEFAULT_COMPILED_CONTEXT_COMPILER: CanonicalCompiledContextCompiler =
    CanonicalCompiledContextCompiler;

impl CompiledContextService {
    pub fn plan<S: CanonicalStore>(
        store: &S,
        index: &DerivedIndex,
        template_ref: &VersionRef,
    ) -> Result<CompiledContextPlan, ProjectError> {
        let template = load_compiled_context_template(store, template_ref)?;
        let selection = collect_selection(store, index, &template)?;
        let mut ids = selection
            .objects
            .iter()
            .map(|s| s.object_id.clone())
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        Ok(CompiledContextPlan {
            template: template_ref.clone(),
            selected_object_ids: ids,
            relation_expansion_count: template.select.relations.len(),
            boundary_relation_count: selection.boundary_relations.len(),
            grouped_by: template.group_by,
            warnings: selection.warnings,
        })
    }

    pub fn compile<S: CanonicalStore>(
        store: &S,
        index: &DerivedIndex,
        template_ref: &VersionRef,
        work_packet: Option<ObjectRef>,
    ) -> Result<WorkSurfaceManifest, ProjectError> {
        let template = load_compiled_context_template(store, template_ref)?;
        let selection = collect_selection(store, index, &template)?;

        let objects = selection
            .objects
            .into_iter()
            .map(|summary| {
                let version = VersionRef::new(
                    earmark_core::ObjectId::parse(summary.object_id.clone())?,
                    earmark_core::VersionId::parse(summary.version_id.clone())?,
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

        let boundary_relations = selection
            .boundary_relations
            .into_iter()
            .map(|candidate| {
                let relation_ref = VersionRef::new(
                    earmark_core::ObjectId::parse(candidate.relation_object_id.clone())?,
                    earmark_core::VersionId::parse(candidate.relation_version_id)?,
                );
                let loaded = store.read_version(&relation_ref)?;
                let payload: earmark_core::RelationPayload =
                    serde_json::from_slice(&loaded.payload.bytes)?;

                let (included_endpoint, excluded_endpoint) =
                    if payload.source.id.as_str() == candidate.included_object_id {
                        (payload.source.clone(), payload.target.clone())
                    } else {
                        (payload.target.clone(), payload.source.clone())
                    };

                if excluded_endpoint.id.as_str() != candidate.excluded_object_id {
                    return Err(ProjectError::Invariant(format!(
                        "Boundary relation {} has unexpected excluded endpoint: {} (expected {})",
                        candidate.relation_object_id,
                        excluded_endpoint.id.as_str(),
                        candidate.excluded_object_id
                    )));
                }

                Ok(WorkSurfaceBoundaryRelation {
                    relation: loaded.envelope.object_ref(),
                    relation_type: candidate.relation_type,
                    source: payload.source,
                    target: payload.target,
                    included_endpoint,
                    excluded_endpoint,
                    path: store.version_path(&relation_ref).display().to_string(),
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
            (
                "expansion_object_filter".to_string(),
                ScalarValue::String(
                    match template.select.expansion.object_filter {
                        ExpansionObjectFilter::Inherit => "inherit",
                        ExpansionObjectFilter::None => "none",
                    }
                    .to_string(),
                ),
            ),
            (
                "include_boundary_relations".to_string(),
                ScalarValue::Bool(template.select.expansion.include_boundary_relations),
            ),
        ]);

        let manifest = WorkSurfaceManifest {
            surface_id: surface_id.clone(),
            compiled_context: template_ref.clone(),
            work_packet,
            generated_at: Utc::now(),
            objects,
            boundary_relations,
            constraints,
            warnings: selection.warnings,
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

#[cfg(test)]
fn collect_selected_objects<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    template: &CompiledContextTemplate,
) -> Result<Vec<ObjectSummary>, ProjectError> {
    Ok(collect_selection(store, index, template)?.objects)
}

fn collect_selection<S: CanonicalStore>(
    _store: &S,
    index: &DerivedIndex,
    template: &CompiledContextTemplate,
) -> Result<CompiledContextSelection, ProjectError> {
    const MAX_RELATION_EXPANSION_DEPTH: usize = 64;
    let enforce_expansion_object_filter = matches!(
        template.select.expansion.object_filter,
        ExpansionObjectFilter::Inherit
    );

    let mut selected = Vec::new();
    let mut seen = BTreeSet::new();
    let mut boundary_relations = Vec::new();
    let mut warnings = Vec::new();

    if !enforce_expansion_object_filter
        && (!template.select.classes.is_empty() || !template.select.standing.is_empty())
    {
        warnings.push("Unfiltered expansion is active while seed class/standing filters are present. Expanded objects will bypass these filters.".to_string());
    }

    let seed_queries: Vec<Option<String>> = if template.select.classes.is_empty() {
        vec![None]
    } else {
        template.select.classes.iter().cloned().map(Some).collect()
    };

    for class in seed_queries {
        let rows = index.query_objects(&QueryFilter {
            class,
            ..Default::default()
        })?;
        for row in rows {
            if object_summary_admissible(&row, &template.select.classes, &template.select.standing)
                && seen.insert((row.object_id.clone(), row.version_id.clone()))
            {
                selected.push(row);
            }
        }
    }

    if !template.select.relations.is_empty() {
        let mut queue = VecDeque::new();
        let mut visited_objects = BTreeSet::new();
        let mut visited_relations = BTreeSet::new();
        let mut additions = Vec::new();

        for row in &selected {
            if visited_objects.insert(row.object_id.clone()) {
                queue.push_back((row.object_id.clone(), 0usize));
            }
        }

        while let Some((current_object_id, depth)) = queue.pop_front() {
            if depth >= MAX_RELATION_EXPANSION_DEPTH {
                continue;
            }

            let mut edges =
                index.relation_adjacency(&ObjectId::parse(current_object_id.clone())?)?;
            edges.sort_by(|a, b| a.version_id.cmp(&b.version_id));
            for edge in edges {
                if !relation_type_admissible(&edge.relation_type, &template.select.relations) {
                    continue;
                }
                if !visited_relations.insert(edge.version_id.clone()) {
                    continue;
                }
                let related_id = if edge.source_object_id == current_object_id {
                    edge.target_object_id
                } else {
                    edge.source_object_id
                };

                let mut related_rows = index.query_objects(&QueryFilter {
                    object_id: Some(related_id.clone()),
                    ..Default::default()
                })?;

                if related_rows.is_empty() {
                    continue;
                }

                related_rows.sort_by(|a, b| a.version_id.cmp(&b.version_id));

                let mut admitted_any = false;
                for related in &related_rows {
                    if enforce_expansion_object_filter
                        && !object_summary_admissible(
                            related,
                            &template.select.classes,
                            &template.select.standing,
                        )
                    {
                        continue;
                    }
                    admitted_any = true;
                    if seen.insert((related.object_id.clone(), related.version_id.clone())) {
                        additions.push(related.clone());
                    }
                }

                if admitted_any {
                    if visited_objects.insert(related_id.clone()) {
                        queue.push_back((related_id, depth + 1));
                    }
                } else if template.select.expansion.include_boundary_relations {
                    boundary_relations.push(BoundaryRelationCandidate {
                        relation_object_id: edge.relation_object_id,
                        relation_version_id: edge.version_id,
                        relation_type: edge.relation_type,
                        included_object_id: current_object_id.clone(),
                        excluded_object_id: related_id,
                    });
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

    Ok(CompiledContextSelection {
        objects: selected,
        boundary_relations,
        warnings,
    })
}

fn render_readme(manifest: &WorkSurfaceManifest) -> String {
    let mut text = format!("# Work Surface {}\n\n", manifest.surface_id);
    for item in &manifest.objects {
        text.push_str(&format!(
            "- {} ({})\n",
            item.title.clone().unwrap_or_else(|| "untitled".to_string()),
            item.object.id.as_str()
        ));
    }

    if !manifest.boundary_relations.is_empty() {
        text.push_str("\n## Boundary relations\n\n");
        for item in &manifest.boundary_relations {
            text.push_str(&format!(
                "- {}: included {} -> excluded {}\n",
                item.relation_type,
                item.included_endpoint.id.as_str(),
                item.excluded_endpoint.id.as_str()
            ));
        }
    }

    text
}

fn render_evidence_pack(manifest: &WorkSurfaceManifest) -> String {
    let mut text = format!("# Evidence Pack {}\n\n", manifest.surface_id);
    for item in &manifest.objects {
        text.push_str(&format!(
            "## {}\n\n- Object: {}\n- Path: {}\n- Version: {}\n\n",
            item.title.clone().unwrap_or_else(|| "untitled".to_string()),
            item.object.id.as_str(),
            item.path,
            item.object.version_id.as_str(),
        ));
    }

    if !manifest.boundary_relations.is_empty() {
        text.push_str("# Boundary Relations\n\n");
        for item in &manifest.boundary_relations {
            text.push_str(&format!(
                "## {}\n\n- Relation: {}\n- Included endpoint: {}\n- Excluded endpoint: {}\n- Path: {}\n\n",
                item.relation_type,
                item.relation.id.as_str(),
                item.included_endpoint.id.as_str(),
                item.excluded_endpoint.id.as_str(),
                item.path
            ));
        }
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
    #[error("invariant violation: {0}")]
    Invariant(String),
}

use earmark_index::IndexError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::object_summary_matches_standing;
    use earmark_core::{
        CompiledContextExpansion, DimensionId, EpistemicStanding, ExpansionObjectFilter, Kind,
        ProcessStanding, Provenance, ReviewStanding, Standing, TokenId,
    };
    use earmark_store::{CanonicalStore, GitCanonicalStore, StoredObject, StoredPayload};
    use tempfile::tempdir;

    #[test]
    fn test_matches_standing() {
        let row = earmark_index::ObjectSummary {
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
            standing: BTreeMap::new(),
        };

        let mut filters = BTreeMap::new();
        filters.insert("review".to_string(), vec!["accepted".to_string()]);
        assert!(object_summary_matches_standing(&row, &filters));

        filters.insert("review".to_string(), vec!["rejected".to_string()]);
        assert!(!object_summary_matches_standing(&row, &filters));

        let empty_filters = BTreeMap::new();
        assert!(object_summary_matches_standing(&row, &empty_filters));
    }

    #[test]
    fn test_matches_standing_unknown_dimension_returns_false() {
        let row = earmark_index::ObjectSummary {
            object_id: "o1".to_string(),
            version_id: "v1".to_string(),
            kind: "Object".to_string(),
            class: None,
            title: None,
            summary: None,
            standing_review: "accepted".to_string(),
            standing_process: "active".to_string(),
            standing_epistemic: "confirmed".to_string(),
            system_id: None,
            namespace: None,
            standing: BTreeMap::new(),
        };
        let mut filters = BTreeMap::new();
        filters.insert("research:status".to_string(), vec!["verified".to_string()]);
        assert!(
            !object_summary_matches_standing(&row, &filters),
            "unknown dimension not in row.standing must return false"
        );
    }

    #[test]
    fn test_matches_standing_legacy_fallback_still_works() {
        let row = earmark_index::ObjectSummary {
            object_id: "o1".to_string(),
            version_id: "v1".to_string(),
            kind: "Object".to_string(),
            class: None,
            title: None,
            summary: None,
            standing_review: "accepted".to_string(),
            standing_process: "active".to_string(),
            standing_epistemic: "confirmed".to_string(),
            system_id: None,
            namespace: None,
            standing: BTreeMap::new(),
        };
        // Legacy short name still falls back to standing_review column
        let filters = BTreeMap::from([("kernel:review".to_string(), vec!["accepted".to_string()])]);
        assert!(object_summary_matches_standing(&row, &filters));
        // Long name also works
        let filters = BTreeMap::from([("kernel:review".to_string(), vec!["rejected".to_string()])]);
        assert!(!object_summary_matches_standing(&row, &filters));
    }

    #[test]
    fn test_matches_standing_empty_allowed_tokens_is_unconstrained() {
        let row = earmark_index::ObjectSummary {
            object_id: "o1".to_string(),
            version_id: "v1".to_string(),
            kind: "Object".to_string(),
            class: None,
            title: None,
            summary: None,
            standing_review: "accepted".to_string(),
            standing_process: "active".to_string(),
            standing_epistemic: "confirmed".to_string(),
            system_id: None,
            namespace: None,
            standing: BTreeMap::from([("research:status".to_string(), "verified".to_string())]),
        };
        let mut filters = BTreeMap::new();
        filters.insert("research:status".to_string(), vec![]);
        assert!(
            object_summary_matches_standing(&row, &filters),
            "empty allowed-token list must be unconstrained"
        );
    }

    #[test]
    fn test_matches_standing_custom_dimension_through_standing_map() {
        let row = earmark_index::ObjectSummary {
            object_id: "o1".to_string(),
            version_id: "v1".to_string(),
            kind: "Object".to_string(),
            class: None,
            title: None,
            summary: None,
            standing_review: "unreviewed".to_string(),
            standing_process: "active".to_string(),
            standing_epistemic: "working".to_string(),
            system_id: None,
            namespace: None,
            standing: BTreeMap::from([("research:status".to_string(), "demonstrated".to_string())]),
        };
        // Match via row.standing
        let filters = BTreeMap::from([(
            "research:status".to_string(),
            vec!["demonstrated".to_string()],
        )]);
        assert!(object_summary_matches_standing(&row, &filters));
        // Non-matching token
        let filters =
            BTreeMap::from([("research:status".to_string(), vec!["verified".to_string()])]);
        assert!(!object_summary_matches_standing(&row, &filters));
    }

    #[test]
    fn test_cli_summary() {
        let manifest = WorkSurfaceManifest {
            surface_id: "test".to_string(),
            compiled_context: VersionRef::new(
                ObjectId::parse("obj_00000000000000000000000000000001").unwrap(),
                earmark_core::VersionId::parse("ver_00000000000000000000000000000001").unwrap(),
            ),
            work_packet: None,
            generated_at: Utc::now(),
            objects: vec![WorkSurfaceObject {
                object: ObjectRef {
                    id: ObjectId::parse("obj_00000000000000000000000000000002").unwrap(),
                    version_id: earmark_core::VersionId::parse(
                        "ver_00000000000000000000000000000002",
                    )
                    .unwrap(),
                    kind: Kind::Object,
                    class: None,
                },
                title: None,
                path: "p".to_string(),
                excerpt_range: None,
                lineage: vec![],
            }],
            boundary_relations: vec![],
            constraints: BTreeMap::new(),
            warnings: vec![],
        };
        assert_eq!(
            CompiledContextService::cli_summary(&manifest),
            "surface test with 1 object(s)"
        );
    }

    fn standing_kernel(epistemic: &str, review: &str, process: &str) -> Standing {
        let mut s = Standing::default();
        s.values.insert(
            DimensionId::new("kernel:epistemic"),
            TokenId::new(epistemic),
        );
        s.values
            .insert(DimensionId::new("kernel:review"), TokenId::new(review));
        s.values
            .insert(DimensionId::new("kernel:process"), TokenId::new(process));
        s
    }

    fn object(title: &str, standing: Standing) -> StoredObject {
        StoredObject::new(
            Kind::Object,
            Some("note".to_string()),
            standing,
            Provenance::direct_input("operator"),
            BTreeMap::from([(
                "title".to_string(),
                earmark_core::HeaderValue::String(title.to_string()),
            )]),
            StoredPayload::from_markdown(title),
            vec![],
        )
    }

    fn relation(source: &StoredObject, target: &StoredObject, relation_type: &str) -> StoredObject {
        StoredObject::new(
            Kind::Relation,
            None,
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_json_bytes(
                serde_json::to_vec(&serde_json::json!({
                    "source": source.object_ref(),
                    "target": target.object_ref(),
                    "relation_type": relation_type,
                    "qualifiers": {},
                    "scope": "test"
                }))
                .unwrap(),
            ),
            vec![],
        )
    }

    fn template_with_standing(standing: BTreeMap<String, Vec<String>>) -> CompiledContextTemplate {
        template_with_select(
            vec!["note".to_string()],
            standing,
            vec!["linked".to_string()],
            None,
        )
    }

    fn template_with_select(
        classes: Vec<String>,
        standing: BTreeMap<String, Vec<String>>,
        relations: Vec<String>,
        expansion: Option<ExpansionObjectFilter>,
    ) -> CompiledContextTemplate {
        CompiledContextTemplate {
            name: "ctx".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            select: earmark_core::CompiledContextSelect {
                classes,
                standing,
                relations,
                time_range: None,
                expansion: CompiledContextExpansion {
                    object_filter: expansion.unwrap_or(ExpansionObjectFilter::Inherit),
                    include_boundary_relations: false,
                },
            },
            group_by: vec![],
            render: earmark_core::CompiledContextRender {
                mode: "manifest".to_string(),
                manifest_format: None,
                prose_template: None,
            },
            visibility: earmark_core::CompiledContextVisibility {
                include_lineage: false,
                include_constraints: false,
                include_provenance: false,
            },
        }
    }

    #[test]
    fn collect_selected_objects_terminates_on_two_node_cycle() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let a = object("a", Standing::default());
        let b = object("b", Standing::default());
        store.write_object(&a).unwrap();
        store.write_object(&b).unwrap();
        store.write_object(&relation(&a, &b, "linked")).unwrap();
        store.write_object(&relation(&b, &a, "linked")).unwrap();
        index.rebuild_from_store(&store).unwrap();

        let selected =
            collect_selected_objects(&store, &index, &template_with_standing(BTreeMap::new()))
                .unwrap();
        let ids = selected
            .iter()
            .map(|r| r.object_id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(a.envelope.id.as_str()));
        assert!(ids.contains(b.envelope.id.as_str()));
    }

    #[test]
    fn collect_selected_objects_handles_larger_cycle() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let a = object("a", Standing::default());
        let b = object("b", Standing::default());
        let c = object("c", Standing::default());
        store.write_object(&a).unwrap();
        store.write_object(&b).unwrap();
        store.write_object(&c).unwrap();
        store.write_object(&relation(&a, &b, "linked")).unwrap();
        store.write_object(&relation(&b, &c, "linked")).unwrap();
        store.write_object(&relation(&c, &a, "linked")).unwrap();
        index.rebuild_from_store(&store).unwrap();

        let selected =
            collect_selected_objects(&store, &index, &template_with_standing(BTreeMap::new()))
                .unwrap();
        let ids = selected
            .iter()
            .map(|r| r.object_id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn collect_selected_objects_dedupes_when_multiple_paths_reach_same_node() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let a = object("a", Standing::default());
        let b = object("b", Standing::default());
        let c = object("c", Standing::default());
        let d = object("d", Standing::default());
        store.write_object(&a).unwrap();
        store.write_object(&b).unwrap();
        store.write_object(&c).unwrap();
        store.write_object(&d).unwrap();
        store.write_object(&relation(&a, &b, "linked")).unwrap();
        store.write_object(&relation(&a, &c, "linked")).unwrap();
        store.write_object(&relation(&b, &d, "linked")).unwrap();
        store.write_object(&relation(&c, &d, "linked")).unwrap();
        index.rebuild_from_store(&store).unwrap();

        let selected =
            collect_selected_objects(&store, &index, &template_with_standing(BTreeMap::new()))
                .unwrap();
        let d_count = selected
            .iter()
            .filter(|row| row.object_id == d.envelope.id.as_str())
            .count();
        assert_eq!(d_count, 1);
    }

    #[test]
    fn collect_selected_objects_preserves_standing_filter_on_seed_selection() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let accepted = object("accepted", standing_kernel("working", "accepted", "active"));
        let rejected = object("rejected", standing_kernel("working", "rejected", "active"));
        store.write_object(&accepted).unwrap();
        store.write_object(&rejected).unwrap();
        store
            .write_object(&relation(&accepted, &rejected, "linked"))
            .unwrap();
        index.rebuild_from_store(&store).unwrap();

        let standing = BTreeMap::from([("review".to_string(), vec!["accepted".to_string()])]);
        let selected =
            collect_selected_objects(&store, &index, &template_with_standing(standing)).unwrap();
        let ids = selected
            .iter()
            .map(|r| r.object_id.clone())
            .collect::<BTreeSet<_>>();
        assert!(ids.contains(accepted.envelope.id.as_str()));
        assert!(!ids.contains(rejected.envelope.id.as_str()));
    }

    #[test]
    fn compiled_context_expansion_excludes_wrong_class_neighbor_by_default() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let finding = StoredObject::new(
            Kind::Object,
            Some("finding".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown("finding"),
            vec![],
        );
        let source_note = StoredObject::new(
            Kind::Object,
            Some("source_note".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown("source"),
            vec![],
        );
        store.write_object(&finding).unwrap();
        store.write_object(&source_note).unwrap();
        store
            .write_object(&relation(&finding, &source_note, "linked"))
            .unwrap();
        index.rebuild_from_store(&store).unwrap();

        let selection = collect_selection(
            &store,
            &index,
            &template_with_select(
                vec!["finding".to_string()],
                BTreeMap::new(),
                vec!["linked".to_string()],
                None,
            ),
        )
        .unwrap();
        let selected = selection.objects;
        let ids = selected
            .iter()
            .map(|r| r.object_id.clone())
            .collect::<BTreeSet<_>>();
        assert!(ids.contains(finding.envelope.id.as_str()));
        assert!(!ids.contains(source_note.envelope.id.as_str()));
    }

    #[test]
    fn compiled_context_expansion_respects_relation_type_filter() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let accepted = object("accepted", standing_kernel("working", "accepted", "active"));
        let rejected = object("rejected", standing_kernel("working", "rejected", "active"));
        store.write_object(&accepted).unwrap();
        store.write_object(&rejected).unwrap();
        store
            .write_object(&relation(&accepted, &rejected, "mentions"))
            .unwrap();
        index.rebuild_from_store(&store).unwrap();

        let standing = BTreeMap::from([("review".to_string(), vec!["accepted".to_string()])]);
        let selection = collect_selection(
            &store,
            &index,
            &template_with_select(
                vec!["note".to_string()],
                standing,
                vec!["derived_from".to_string()],
                Some(ExpansionObjectFilter::None),
            ),
        )
        .unwrap();
        let selected = selection.objects;

        let ids = selected
            .iter()
            .map(|r| r.object_id.clone())
            .collect::<BTreeSet<_>>();
        assert!(ids.contains(accepted.envelope.id.as_str()));
        assert!(!ids.contains(rejected.envelope.id.as_str()));
    }

    #[test]
    fn compiled_context_expansion_object_filter_none_includes_rejected_neighbor() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let accepted = object("accepted", standing_kernel("working", "accepted", "active"));
        let rejected = object("rejected", standing_kernel("working", "rejected", "active"));
        store.write_object(&accepted).unwrap();
        store.write_object(&rejected).unwrap();
        store
            .write_object(&relation(&accepted, &rejected, "linked"))
            .unwrap();
        index.rebuild_from_store(&store).unwrap();

        let standing = BTreeMap::from([("review".to_string(), vec!["accepted".to_string()])]);
        let selection = collect_selection(
            &store,
            &index,
            &template_with_select(
                vec!["note".to_string()],
                standing,
                vec!["linked".to_string()],
                Some(ExpansionObjectFilter::None),
            ),
        )
        .unwrap();
        let selected = selection.objects;
        let ids = selected
            .iter()
            .map(|r| r.object_id.clone())
            .collect::<BTreeSet<_>>();
        assert!(ids.contains(accepted.envelope.id.as_str()));
        assert!(ids.contains(rejected.envelope.id.as_str()));

        assert!(!selection.warnings.is_empty());
        assert!(selection.warnings[0].contains("Unfiltered expansion is active"));
    }

    #[test]
    fn compiled_context_does_not_enqueue_filtered_neighbor_for_further_traversal() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let accepted = object("accepted", standing_kernel("working", "accepted", "active"));
        let rejected = object("rejected", standing_kernel("working", "rejected", "active"));
        let hidden_parent = StoredObject::new(
            Kind::Object,
            Some("source_note".to_string()),
            standing_kernel("working", "accepted", "active"),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown("hidden_parent"),
            vec![],
        );

        store.write_object(&accepted).unwrap();
        store.write_object(&rejected).unwrap();
        store.write_object(&hidden_parent).unwrap();
        store
            .write_object(&relation(&accepted, &rejected, "linked"))
            .unwrap();
        store
            .write_object(&relation(&rejected, &hidden_parent, "linked"))
            .unwrap();
        index.rebuild_from_store(&store).unwrap();

        let standing = BTreeMap::from([("review".to_string(), vec!["accepted".to_string()])]);
        let selected =
            collect_selected_objects(&store, &index, &template_with_standing(standing)).unwrap();
        let ids = selected
            .iter()
            .map(|r| r.object_id.clone())
            .collect::<BTreeSet<_>>();
        assert!(ids.contains(accepted.envelope.id.as_str()));
        assert!(!ids.contains(rejected.envelope.id.as_str()));
        assert!(!ids.contains(hidden_parent.envelope.id.as_str()));
    }

    #[test]
    fn test_boundary_relations_omitted_by_default() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let finding = StoredObject::new(
            Kind::Object,
            Some("finding".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown("finding"),
            vec![],
        );
        let source_note = StoredObject::new(
            Kind::Object,
            Some("source_note".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown("source"),
            vec![],
        );
        store.write_object(&finding).unwrap();
        store.write_object(&source_note).unwrap();
        store
            .write_object(&relation(&finding, &source_note, "linked"))
            .unwrap();
        index.rebuild_from_store(&store).unwrap();

        let selection = collect_selection(
            &store,
            &index,
            &template_with_select(
                vec!["finding".to_string()],
                BTreeMap::new(),
                vec!["linked".to_string()],
                None,
            ),
        )
        .unwrap();

        assert_eq!(selection.objects.len(), 1);
        assert_eq!(selection.boundary_relations.len(), 0);
    }

    #[test]
    fn test_compile_includes_boundary_relations() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let finding = StoredObject::new(
            Kind::Object,
            Some("finding".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown("finding"),
            vec![],
        );
        let source_note = StoredObject::new(
            Kind::Object,
            Some("source_note".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown("source"),
            vec![],
        );
        store.write_object(&finding).unwrap();
        store.write_object(&source_note).unwrap();
        let rel_ref = store
            .write_object(&relation(&finding, &source_note, "linked"))
            .unwrap();

        let mut template = template_with_select(
            vec!["finding".to_string()],
            BTreeMap::new(),
            vec!["linked".to_string()],
            None,
        );
        template.select.expansion.include_boundary_relations = true;

        let template_obj = StoredObject::new(
            Kind::CompiledContextTemplate,
            None,
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_json_bytes(serde_json::to_vec(&template).unwrap()),
            vec![],
        );
        let template_ref = store.write_object(&template_obj).unwrap();

        index.rebuild_from_store(&store).unwrap();

        let manifest =
            CompiledContextService::compile(&store, &index, &template_ref, None).unwrap();

        assert_eq!(manifest.objects.len(), 1);
        assert_eq!(manifest.boundary_relations.len(), 1);
        assert_eq!(manifest.boundary_relations[0].relation.id, rel_ref.id);
        assert_eq!(
            manifest.boundary_relations[0].included_endpoint.id,
            finding.envelope.id
        );
        assert_eq!(
            manifest.boundary_relations[0].excluded_endpoint.id,
            source_note.envelope.id
        );

        let evidence = render_evidence_pack(&manifest);
        assert!(evidence.contains("# Boundary Relations"));
        assert!(evidence.contains("## linked"));
    }

    #[test]
    fn test_inbound_relation_is_not_boundary() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let a = object("a", Standing::default());
        let b = object("b", Standing::default());
        store.write_object(&a).unwrap();
        store.write_object(&b).unwrap();
        store.write_object(&relation(&a, &b, "linked")).unwrap();
        index.rebuild_from_store(&store).unwrap();

        let mut template = template_with_standing(BTreeMap::new());
        template.select.expansion.include_boundary_relations = true;

        let selection = collect_selection(&store, &index, &template).unwrap();

        assert_eq!(selection.objects.len(), 2);
        assert_eq!(selection.boundary_relations.len(), 0);
    }

    #[test]
    fn test_wrong_relation_type_no_boundary() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let finding = StoredObject::new(
            Kind::Object,
            Some("finding".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown("finding"),
            vec![],
        );
        let source_note = StoredObject::new(
            Kind::Object,
            Some("source_note".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown("source"),
            vec![],
        );
        store.write_object(&finding).unwrap();
        store.write_object(&source_note).unwrap();
        // Relation type "mentions" which is NOT in the default template's relations ("linked")
        store
            .write_object(&relation(&finding, &source_note, "mentions"))
            .unwrap();
        index.rebuild_from_store(&store).unwrap();

        let mut template = template_with_select(
            vec!["finding".to_string()],
            BTreeMap::new(),
            vec!["linked".to_string()],
            None,
        );
        template.select.expansion.include_boundary_relations = true;

        let selection = collect_selection(&store, &index, &template).unwrap();

        assert_eq!(selection.objects.len(), 1);
        assert_eq!(selection.boundary_relations.len(), 0);
    }

    #[test]
    fn test_filtered_neighbor_not_traversed() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let accepted = object("accepted", standing_kernel("working", "accepted", "active"));
        let rejected = object("rejected", standing_kernel("working", "rejected", "active"));
        let second_hop = object("second_hop", Standing::default());

        store.write_object(&accepted).unwrap();
        store.write_object(&rejected).unwrap();
        store.write_object(&second_hop).unwrap();

        store
            .write_object(&relation(&accepted, &rejected, "linked"))
            .unwrap();
        store
            .write_object(&relation(&rejected, &second_hop, "linked"))
            .unwrap();
        index.rebuild_from_store(&store).unwrap();

        let standing = BTreeMap::from([("review".to_string(), vec!["accepted".to_string()])]);
        let mut template = template_with_standing(standing);
        template.select.expansion.include_boundary_relations = true;

        let selection = collect_selection(&store, &index, &template).unwrap();

        assert_eq!(selection.objects.len(), 1);
        assert_eq!(selection.boundary_relations.len(), 1);
        // Ensure second_hop is not admitted even though it matches the filter,
        // because we stopped at the filtered neighbor (rejected).
        assert!(!selection
            .objects
            .iter()
            .any(|o| o.title == Some("second_hop".to_string())));
    }

    #[test]
    fn test_no_payload_leakage() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let finding = StoredObject::new(
            Kind::Object,
            Some("finding".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown("finding"),
            vec![],
        );
        let secret_payload = "SECRET_UNADMITTED_TEXT_XYZ";
        let excluded = StoredObject::new(
            Kind::Object,
            Some("excluded".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown(secret_payload),
            vec![],
        );
        store.write_object(&finding).unwrap();
        store.write_object(&excluded).unwrap();
        store
            .write_object(&relation(&finding, &excluded, "linked"))
            .unwrap();

        let mut template = template_with_select(
            vec!["finding".to_string()],
            BTreeMap::new(),
            vec!["linked".to_string()],
            None,
        );
        template.select.expansion.include_boundary_relations = true;

        let template_obj = StoredObject::new(
            Kind::CompiledContextTemplate,
            None,
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_json_bytes(serde_json::to_vec(&template).unwrap()),
            vec![],
        );
        let template_ref = store.write_object(&template_obj).unwrap();
        index.rebuild_from_store(&store).unwrap();

        let manifest =
            CompiledContextService::compile(&store, &index, &template_ref, None).unwrap();
        let evidence = render_evidence_pack(&manifest);

        assert!(evidence.contains("# Boundary Relations"));
        assert!(!evidence.contains(secret_payload));
    }
}
