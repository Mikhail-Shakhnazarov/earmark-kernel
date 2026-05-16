use crate::app::common::CliError;
use crate::app::listing::list_run_records;
use earmark_core::{ObjectRef, VersionRef};
use earmark_index::DerivedIndex;
use earmark_store::CanonicalStore;

pub(crate) fn resolve_object_ref<S: CanonicalStore>(
    store: &S,
    object_id: &str,
) -> Result<ObjectRef, CliError> {
    let head = store
        .read_head(&earmark_core::ObjectId::parse(object_id.to_string())?)?
        .ok_or_else(|| CliError::not_found(format!("object not found: {}", object_id)))?;
    Ok(head.object_ref())
}

pub(crate) fn resolve_run_id<S: CanonicalStore>(store: &S, run_id: &str) -> Result<String, CliError> {
    if run_id == "latest" {
        let ledgers = list_run_records(store)?;
        return ledgers
            .last()
            .map(|l| l.run_id.clone())
            .ok_or_else(|| CliError::not_found("no runs found".to_string()));
    }
    Ok(run_id.to_string())
}

pub(crate) fn resolve_optional_run_id<S: CanonicalStore>(
    store: &S,
    run_id: Option<String>,
) -> Result<Option<String>, CliError> {
    match run_id {
        Some(id) => Ok(Some(resolve_run_id(store, &id)?)),
        None => Ok(None),
    }
}

pub(crate) fn resolve_system_version_ref(
    index: &DerivedIndex,
    system_id: &str,
) -> Result<VersionRef, CliError> {
    let found = index.find_system_definition(system_id)?.ok_or_else(|| {
        CliError::not_found(format!("system definition not found: {}", system_id))
    })?;
    Ok(VersionRef::new(
        earmark_core::ObjectId::parse(found.0)?,
        earmark_core::VersionId::parse(found.1)?,
    ))
}
