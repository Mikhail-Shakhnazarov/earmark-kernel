use crate::app::common::{CliError, CommandContext};
use crate::app::emit;
use crate::cli::{ContextAction, ContextCommand};
use earmark_core::{ClassFilter, DimensionId, RelationFilter, StandingFilter, TokenId};
use earmark_runtime_tools::RuntimeToolSurface;
use std::collections::BTreeMap;

pub fn handle(ctx: &CommandContext, command: &ContextCommand) -> Result<(), CliError> {
    let store = ctx.store;
    let index = ctx.index.as_ref().expect("index required for context");
    let provider_registry = ctx.provider_registry;
    let as_json = ctx.as_json;

    let runtime_surface = RuntimeToolSurface {
        store,
        index,
        provider_service: provider_registry,
    };

    match &command.action {
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
                .clone()
                .into_iter()
                .map(earmark_core::ObjectId::parse)
                .collect::<Result<Vec<_>, _>>()?;
            let manifest = runtime_surface.compile_connected_context(
                roots,
                args.depth,
                Some(RelationFilter {
                    allowed_types: args.relation_types.clone(),
                }),
                Some(ClassFilter {
                    allowed_classes: args.classes.clone(),
                }),
                standing_filter,
            )?;
            emit(as_json, serde_json::to_value(manifest)?);
        }
    }
    Ok(())
}
