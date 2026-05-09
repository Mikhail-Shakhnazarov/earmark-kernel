use earmark_core::{ClassFilter, RelationFilter, StandingFilter};
use earmark_runtime_tools::RuntimeToolSurface;
use earmark_store::GitCanonicalStore;

use crate::app::{emit, parse_epistemic, CliError};
use crate::cli::{ContextAction, ContextCommand};

pub fn handle(
    runtime_surface: &RuntimeToolSurface<'_, GitCanonicalStore>,
    as_json: bool,
    command: ContextCommand,
) -> Result<(), CliError> {
    match command.action {
        ContextAction::Compile(args) => {
            let standing_filter = if args.epistemic.is_empty() {
                None
            } else {
                Some(StandingFilter {
                    allowed_epistemic: args
                        .epistemic
                        .iter()
                        .map(|value| parse_epistemic(value))
                        .collect::<Result<Vec<_>, _>>()?,
                })
            };
            let roots = args
                .roots
                .into_iter()
                .map(earmark_core::ObjectId::parse)
                .collect::<Result<Vec<_>, _>>()?;
            let manifest = runtime_surface.compile_connected_context(
                roots,
                args.depth,
                Some(RelationFilter {
                    allowed_types: args.relation_types,
                }),
                Some(ClassFilter {
                    allowed_classes: args.classes,
                }),
                standing_filter,
            )?;
            emit(as_json, serde_json::to_value(manifest)?);
        }
    }
    Ok(())
}
