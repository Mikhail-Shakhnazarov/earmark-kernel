use crate::modules::error::RuntimeToolError;
use crate::modules::surface::RuntimeToolSurface;
use earmark_connected_context::{
    CompiledContextCompiler, WorkSurfaceManifest, DEFAULT_COMPILED_CONTEXT_COMPILER,
};
use earmark_core::{
    ClassFilter, ConnectedContextManifest, Kind, ObjectId, StandingFilter, VersionRef,
};
use earmark_store::{CanonicalStore, StoredObject};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

impl<'a, S: CanonicalStore> RuntimeToolSurface<'a, S> {
    pub fn compile_work_surface(
        &self,
        compiled_context_ref: &VersionRef,
    ) -> Result<WorkSurfaceManifest, RuntimeToolError> {
        self.compile_work_surface_with(&DEFAULT_COMPILED_CONTEXT_COMPILER, compiled_context_ref)
    }

    pub fn compile_work_surface_with<C: CompiledContextCompiler<S>>(
        &self,
        context_compiler: &C,
        compiled_context_ref: &VersionRef,
    ) -> Result<WorkSurfaceManifest, RuntimeToolError> {
        Ok(context_compiler.compile(self.store, self.index, compiled_context_ref, None)?)
    }

    pub fn compile_connected_context(
        &self,
        root_object_ids: Vec<ObjectId>,
        max_depth: usize,
        relation_filter: Option<earmark_core::RelationFilter>,
        class_filter: Option<ClassFilter>,
        standing_filter: Option<StandingFilter>,
    ) -> Result<ConnectedContextManifest, RuntimeToolError> {
        let heads = current_head_objects(self.store)?;

        let mut queue = VecDeque::new();
        let mut seen_objects = BTreeSet::new();
        let mut seen_relations = BTreeSet::new();
        let mut object_refs = Vec::new();
        let mut relation_refs = Vec::new();

        for root_id in &root_object_ids {
            let stored = heads
                .get(root_id)
                .ok_or_else(|| RuntimeToolError::MissingObject(root_id.as_str().to_string()))?;
            seen_objects.insert(root_id.clone());
            object_refs.push(stored.object_ref());
            queue.push_back((root_id.clone(), 0usize));
        }

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            for relation in heads
                .values()
                .filter(|obj| obj.envelope.kind == Kind::Relation)
            {
                let payload: earmark_core::RelationPayload =
                    serde_json::from_slice(&relation.payload.bytes)?;
                if !crate::modules::relations::relation_type_allowed(
                    &payload.relation_type,
                    relation_filter.as_ref(),
                ) {
                    continue;
                }

                let neighbor_id = if payload.source.id == current_id {
                    Some(payload.target.id.clone())
                } else if payload.target.id == current_id {
                    Some(payload.source.id.clone())
                } else {
                    None
                };

                let Some(neighbor_id) = neighbor_id else {
                    continue;
                };
                let Some(neighbor) = heads.get(&neighbor_id) else {
                    continue;
                };
                if !object_allowed(neighbor, class_filter.as_ref(), standing_filter.as_ref()) {
                    continue;
                }

                if seen_relations.insert(relation.envelope.id.clone()) {
                    relation_refs.push(relation.object_ref());
                }
                if seen_objects.insert(neighbor_id.clone()) {
                    object_refs.push(neighbor.object_ref());
                    queue.push_back((neighbor_id, depth + 1));
                }
            }
        }

        Ok(ConnectedContextManifest {
            root_object_ids,
            object_refs,
            relation_refs,
            max_depth,
            generated_at: chrono::Utc::now(),
        })
    }
}

pub(crate) fn current_head_objects<S: CanonicalStore>(
    store: &S,
) -> Result<BTreeMap<ObjectId, StoredObject>, RuntimeToolError> {
    let mut heads = BTreeMap::new();
    for object in store.scan_objects()? {
        if let Some(head_ref) = store.read_head_ref(&object.envelope.id)? {
            if head_ref.version_id == object.envelope.version_id {
                heads.insert(object.envelope.id.clone(), object);
            }
        }
    }
    Ok(heads)
}

pub(crate) fn object_allowed(
    object: &StoredObject,
    class_filter: Option<&ClassFilter>,
    standing_filter: Option<&StandingFilter>,
) -> bool {
    let class_ok = class_filter
        .map(|filter| {
            filter.allowed_classes.is_empty()
                || object
                    .envelope
                    .class
                    .as_ref()
                    .map(|class| {
                        filter
                            .allowed_classes
                            .iter()
                            .any(|allowed| allowed == class)
                    })
                    .unwrap_or(false)
        })
        .unwrap_or(true);
    let standing_ok = standing_filter
        .map(|filter| {
            filter.allowed.is_empty()
                || filter.allowed.iter().all(|(dim_id, allowed_tokens)| {
                    let actual = object.envelope.standing.get(dim_id);
                    allowed_tokens.is_empty()
                        || actual
                            .map(|t| allowed_tokens.iter().any(|a| a == t))
                            .unwrap_or(false)
                })
        })
        .unwrap_or(true);
    class_ok && standing_ok
}
