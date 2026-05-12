use crate::app::common::{CliError, CommandContext};
use crate::app::{emit, mirror_surface, resolve_version_ref};
use crate::cli::ReviewArgs;
use earmark_governance::GovernanceService;
use earmark_store::CanonicalStore;
use serde_json::json;

pub fn handle(ctx: &CommandContext, args: &ReviewArgs) -> Result<(), CliError> {
    let store = ctx.store;
    let index = ctx.index.as_ref().expect("index required for review");
    let as_json = ctx.as_json;

    let reference = resolve_version_ref(store, &args.object_id, args.version_id.as_deref())?;
    let target_object = store.read_version(&reference)?;
    let review = GovernanceService::create_review_object(
        target_object.object_ref(),
        !args.reject,
        args.reason.clone(),
    )?;
    mirror_surface(store, &review)?;
    earmark_exec::persistence_helpers::write_object_and_index(store, index, &review)?;
    emit(
        as_json,
        json!({
            "ok": true,
            "review_object_id": review.envelope.id.as_str(),
            "review_version_id": review.envelope.version_id.as_str(),
            "target_object_id": target_object.envelope.id.as_str(),
            "status": if args.reject { "rejected" } else { "accepted" },
        }),
    );
    Ok(())
}
