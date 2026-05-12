use crate::app::common::{CliError, CommandContext};
use crate::app::emit;
use crate::cli::QueryArgs;
use earmark_index::QueryFilter;

pub fn handle(ctx: &CommandContext, args: &QueryArgs) -> Result<(), CliError> {
    let index = ctx.index.as_ref().expect("index required for query");
    let as_json = ctx.as_json;

    let rows = index.query_objects(&QueryFilter {
        class: args.class.clone(),
        kind: args.kind.clone(),
        text: args.text.clone(),
        object_id: args.object_id.clone(),
        ..Default::default()
    })?;
    emit(as_json, serde_json::to_value(rows)?);
    Ok(())
}
