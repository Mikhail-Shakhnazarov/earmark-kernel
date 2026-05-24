use crate::app::common::{CliError, CommandContext};
use crate::app::{emit, mirror_surface};
use crate::cli::DepositArgs;
use crate::config::resolve_system_id;
use earmark_core::{Kind, VersionRef};
use earmark_runtime_tools::RuntimeToolSurface;
use earmark_store::ObjectStore;
use serde_json::json;
use std::fs;

pub fn handle(ctx: &CommandContext, args: &DepositArgs) -> Result<(), CliError> {
    let store = ctx.store;
    let index = ctx.index.as_ref().ok_or_else(|| {
        CliError::argument("index required for deposit — ensure workspace is initialized")
    })?;
    let provider_registry = ctx.provider_registry;
    let config = ctx.config;
    let as_json = ctx.as_json;

    let runtime_surface = RuntimeToolSurface {
        store,
        index,
        provider_service: provider_registry,
    };

    let mut validation_context = earmark_runtime_tools::DepositValidationContext::default();

    if let Some(system_id) = resolve_system_id(None, config) {
        if let Some((_obj_id, _version_id, namespace)) =
            runtime_surface.index.find_system_definition(&system_id)?
        {
            validation_context.namespace = Some(namespace);
        } else {
            return Err(
                earmark_runtime_tools::RuntimeToolError::SystemIntegrity(format!(
                "configured system context '{}' not found in index; cannot ensure admission safety",
                system_id
            ))
                .into(),
            );
        }
    }

    let kind: Kind = args.kind.parse()?;
    let text = if let Some(path) = &args.payload_file {
        fs::read_to_string(path)?
    } else if let Some(raw_json) = &args.json_payload {
        raw_json.clone()
    } else {
        args.body.clone().unwrap_or_default()
    };

    let payload_value = if args.json_payload.is_some() || kind == Kind::Relation {
        serde_json::from_str(&text)?
    } else {
        json!(text)
    };

    let prov = earmark_core::RuntimeProvenance {
        actor: "operator".to_string(),
        source_type: "cli".to_string(),
    };

    for h in &args.headers {
        if let Some((k, v)) = h.split_once('=') {
            validation_context.headers.insert(
                k.to_string(),
                earmark_core::HeaderValue::String(v.to_string()),
            );
        } else {
            return Err(CliError::argument(format!(
                "invalid header format '{}', expected key=value",
                h
            )));
        }
    }

    let object_ref = runtime_surface.deposit_object(
        args.class.clone(),
        Some(args.kind.clone()),
        args.title.clone(),
        payload_value,
        prov,
        validation_context,
    )?;

    let reference = VersionRef::new(object_ref.id.clone(), object_ref.version_id.clone());
    let object = store.read_version(&reference)?;
    mirror_surface(store, &object)?;

    emit(
        as_json,
        json!({
            "object_id": object_ref.id.as_str(),
            "version_id": object_ref.version_id.as_str(),
            "kind": object_ref.kind.as_str(),
            "class": object_ref.class,
            "title": args.title,
        }),
    );
    Ok(())
}
