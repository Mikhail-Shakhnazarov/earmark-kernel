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
    const MAX_RELATION_EXPANSION_DEPTH: usize = 64;
    let enforce_expansion_object_filter = matches!(
        template.select.expansion.object_filter,
        ExpansionObjectFilter::Inherit
    );

    let mut selected = Vec::new();
    let mut seen = BTreeSet::new();

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
                if !visited_objects.insert(related_id.clone()) {
                    continue;
                }

                let mut related_rows = index.query_objects(&QueryFilter {
                    object_id: Some(related_id.clone()),
                    ..Default::default()
                })?;
                related_rows.sort_by(|a, b| a.version_id.cmp(&b.version_id));
                let mut admitted_any = false;
                for related in related_rows {
                    if enforce_expansion_object_filter
                        && !object_summary_admissible(
                            &related,
                            &template.select.classes,
                            &template.select.standing,
                        )
                    {
                        continue;
                    }
                    admitted_any = true;
                    if seen.insert((related.object_id.clone(), related.version_id.clone())) {
                        additions.push(related);
                    }
                }
                if admitted_any {
                    queue.push_back((related_id, depth + 1));
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

fn render_readme(manifest: &WorkSurfaceManifest) -> String {
    let mut text = format!("# Work Surface {}\n\n", manifest.surface_id);
    for item in &manifest.objects {
        text.push_str(&format!(
            "- {} ({})\n",
            item.title.clone().unwrap_or_else(|| "untitled".to_string()),
            item.object.id.as_str()
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
            item.object.id.as_str(),
            item.path,
            item.object.version_id.as_str(),
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
    use crate::filter::object_summary_matches_standing;
    use earmark_core::{
        CompiledContextExpansion, EpistemicStanding, ExpansionObjectFilter, Kind, ProcessStanding,
        Provenance, ReviewStanding, Standing,
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
            constraints: BTreeMap::new(),
        };
        assert_eq!(
            CompiledContextService::cli_summary(&manifest),
            "surface test with 1 object(s)"
        );
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

        let accepted = object(
            "accepted",
            Standing {
                epistemic: EpistemicStanding::Working,
                review: ReviewStanding::Accepted,
                process: ProcessStanding::Active,
            },
        );
        let rejected = object(
            "rejected",
            Standing {
                epistemic: EpistemicStanding::Working,
                review: ReviewStanding::Rejected,
                process: ProcessStanding::Active,
            },
        );
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

        let selected = collect_selected_objects(
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

        let accepted = object(
            "accepted",
            Standing {
                epistemic: EpistemicStanding::Working,
                review: ReviewStanding::Accepted,
                process: ProcessStanding::Active,
            },
        );
        let rejected = object(
            "rejected",
            Standing {
                epistemic: EpistemicStanding::Working,
                review: ReviewStanding::Rejected,
                process: ProcessStanding::Active,
            },
        );
        store.write_object(&accepted).unwrap();
        store.write_object(&rejected).unwrap();
        store
            .write_object(&relation(&accepted, &rejected, "mentions"))
            .unwrap();
        index.rebuild_from_store(&store).unwrap();

        let standing = BTreeMap::from([("review".to_string(), vec!["accepted".to_string()])]);
        let selected = collect_selected_objects(
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

        let accepted = object(
            "accepted",
            Standing {
                epistemic: EpistemicStanding::Working,
                review: ReviewStanding::Accepted,
                process: ProcessStanding::Active,
            },
        );
        let rejected = object(
            "rejected",
            Standing {
                epistemic: EpistemicStanding::Working,
                review: ReviewStanding::Rejected,
                process: ProcessStanding::Active,
            },
        );
        store.write_object(&accepted).unwrap();
        store.write_object(&rejected).unwrap();
        store
            .write_object(&relation(&accepted, &rejected, "linked"))
            .unwrap();
        index.rebuild_from_store(&store).unwrap();

        let standing = BTreeMap::from([("review".to_string(), vec!["accepted".to_string()])]);
        let selected = collect_selected_objects(
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
        let ids = selected
            .iter()
            .map(|r| r.object_id.clone())
            .collect::<BTreeSet<_>>();
        assert!(ids.contains(accepted.envelope.id.as_str()));
        assert!(ids.contains(rejected.envelope.id.as_str()));
    }

    #[test]
    fn compiled_context_does_not_enqueue_filtered_neighbor_for_further_traversal() {
        let dir = tempdir().unwrap();
        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let accepted = object(
            "accepted",
            Standing {
                epistemic: EpistemicStanding::Working,
                review: ReviewStanding::Accepted,
                process: ProcessStanding::Active,
            },
        );
        let rejected = object(
            "rejected",
            Standing {
                epistemic: EpistemicStanding::Working,
                review: ReviewStanding::Rejected,
                process: ProcessStanding::Active,
            },
        );
        let hidden_parent = StoredObject::new(
            Kind::Object,
            Some("source_note".to_string()),
            Standing {
                epistemic: EpistemicStanding::Working,
                review: ReviewStanding::Accepted,
                process: ProcessStanding::Active,
            },
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
    fn compiled_context_existing_yaml_without_expansion_defaults_to_inherit() {
        let parsed: CompiledContextTemplate = parse_yaml(
            r#"
name: findings_for_summary
version: 0.2.0
description: Compile findings for summarization.
select:
  classes:
    - finding
  standing: {}
  relations:
    - derived_from
  time_range: null
group_by: []
render:
  mode: work_surface_compilation
  manifest_format: json
  prose_template: null
visibility:
  include_lineage: true
  include_constraints: true
  include_provenance: true
"#,
        )
        .unwrap();
        assert_eq!(
            parsed.select.expansion.object_filter,
            ExpansionObjectFilter::Inherit
        );
        assert!(!parsed.select.expansion.include_boundary_relations);
    }
}
