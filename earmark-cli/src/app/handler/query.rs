use earmark_index::{DerivedIndex, QueryFilter};

use crate::app::{emit, CliError};
use crate::cli::QueryArgs;

pub fn handle(index: &DerivedIndex, as_json: bool, args: QueryArgs) -> Result<(), CliError> {
    let rows = index.query_objects(&QueryFilter {
        class: args.class,
        kind: args.kind,
        text: args.text,
        object_id: args.object_id,
        ..Default::default()
    })?;
    emit(as_json, serde_json::to_value(rows)?);
    Ok(())
}
