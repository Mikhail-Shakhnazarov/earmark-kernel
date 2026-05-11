use std::collections::BTreeMap;

use earmark_core::{ClassFilter, DimensionId, RelationFilter, StandingFilter, TokenId};
use earmark_runtime_tools::RuntimeToolSurface;
use earmark_store::GitCanonicalStore;

use crate::app::{emit, CliError};
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
                let mut allowed = BTreeMap::new();
                allowed.insert(
                    DimensionId::new("kernel:epistemic"),
                    args.epistemic
                        .iter()
                        .map(|value| TokenId::new(value.as_str()))
                        .collect(),
                );
                Some(StandingFilter { allowed })
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
