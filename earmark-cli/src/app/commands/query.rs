use crate::app::common::{CliError, CommandContext};
use crate::app::emit;
use crate::cli::QueryArgs;
use earmark_index::QueryFilter;
use serde_json::json;

pub fn handle(ctx: &mut CommandContext, args: &QueryArgs) -> Result<(), CliError> {
    let index = ctx.index.as_mut().ok_or_else(|| {
        CliError::argument("index required for query — ensure workspace is initialized")
    })?;
    let as_json = ctx.as_json;

    let rows = index.query_objects(&QueryFilter {
        class: args.class.clone(),
        kind: args.kind.clone(),
        text: args.text.clone(),
        object_id: args.object_id.clone(),
        ..Default::default()
    })?;
    emit(
        as_json,
        json!({
            "kind": "query_results",
            "id": "search",
            "summary": format!("{} objects matched the query", rows.len()),
            "results": rows,
        }),
    );
    Ok(())
}
