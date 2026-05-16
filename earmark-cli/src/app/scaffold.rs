use std::path::PathBuf;
use std::{fs, path::Path};

use crate::app::common::CliError;
use crate::cli::DeclarationKind;
use earmark_core::Kind;
use earmark_store::{GitCanonicalStore, PayloadEncoding, StoredObject, WorkspaceLayout};

pub(crate) fn template_contents_for_kind(kind: DeclarationKind) -> &'static str {
    match kind {
        DeclarationKind::Class => include_str!("../../../templates/classes/class.yaml"),
        DeclarationKind::Instruction => {
            include_str!("../../../templates/instructions/instruction.md")
        }
        DeclarationKind::StandingPolicy => {
            include_str!("../../../templates/standing_policies/standing_policy.yaml")
        }
        DeclarationKind::CompiledContext => {
            include_str!("../../../templates/compiled_contexts/compiled_context.yaml")
        }
        DeclarationKind::ProviderProfile => {
            include_str!("../../../templates/provider_profiles/provider_profile.yaml")
        }
        DeclarationKind::Workflow => include_str!("../../../templates/workflows/workflow.yaml"),
        DeclarationKind::System => {
            include_str!("../../../templates/systems/system_path_manifest.yaml")
        }
    }
}

fn default_output_path(root: &Path, kind: DeclarationKind, name: &str) -> PathBuf {
    let (dir, ext) = match kind {
        DeclarationKind::Class => ("declarations/classes", "yaml"),
        DeclarationKind::Instruction => ("declarations/instructions", "md"),
        DeclarationKind::StandingPolicy => ("declarations/standing_policies", "yaml"),
        DeclarationKind::CompiledContext => ("declarations/compiled_contexts", "yaml"),
        DeclarationKind::ProviderProfile => ("declarations/provider_profiles", "yaml"),
        DeclarationKind::Workflow => ("declarations/workflows", "yaml"),
        DeclarationKind::System => ("declarations/systems", "yaml"),
    };
    root.join(dir).join(format!("{name}.{ext}"))
}

pub(crate) fn scaffold_declaration(
    root: &Path,
    kind: DeclarationKind,
    name: &str,
    explicit_path: Option<&PathBuf>,
    force: bool,
) -> Result<PathBuf, CliError> {
    let mut body = template_contents_for_kind(kind).to_string();
    body = body
        .replace("your_class_name", name)
        .replace("your_instruction_name", name)
        .replace("your_standing_policy", name)
        .replace("your_compiled_context", name)
        .replace("your_provider_profile", name)
        .replace("your_workflow", name)
        .replace("your_system", name);

    let out_path = explicit_path
        .cloned()
        .unwrap_or_else(|| default_output_path(root, kind, name));
    if out_path.exists() && !force {
        return Err(CliError::argument(format!(
            "target already exists: {} (pass --force to overwrite)",
            out_path.display()
        )));
    }
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out_path, body)?;
    Ok(out_path)
}

pub(crate) fn collect_paths_with_extensions(
    root: &Path,
    extensions: &[&str],
    out: &mut Vec<String>,
) -> Result<(), CliError> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_paths_with_extensions(&path, extensions, out)?;
            continue;
        }
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if !extensions
            .iter()
            .any(|candidate| ext.eq_ignore_ascii_case(candidate))
        {
            continue;
        }
        out.push(path.display().to_string());
    }
    Ok(())
}

pub(crate) fn mirror_surface(
    store: &GitCanonicalStore,
    object: &StoredObject,
) -> Result<(), CliError> {
    let (dir, ext) = match &object.envelope.kind {
        Kind::Instruction => (
            store.declarations_dir().join("instructions"),
            object.payload.format.extension(),
        ),
        Kind::Workflow => (
            store.declarations_dir().join("workflows"),
            object.payload.format.extension(),
        ),
        Kind::Policy => (
            store.declarations_dir().join("standing_policies"),
            object.payload.format.extension(),
        ),
        Kind::CompiledContextTemplate => (
            store.declarations_dir().join("compiled_contexts"),
            object.payload.format.extension(),
        ),
        Kind::ProviderProfile => (
            store.declarations_dir().join("provider_profiles"),
            object.payload.format.extension(),
        ),
        Kind::SystemDefinition => (
            store.declarations_dir().join("systems"),
            object.payload.format.extension(),
        ),
        Kind::Object | Kind::Review
            if matches!(object.payload.format, PayloadEncoding::Markdown) =>
        {
            (store.corpus_dir(), object.payload.format.extension())
        }
        _ => (
            store
                .root()
                .join(".earmark")
                .join("canonical")
                .join("mirrors"),
            object.payload.format.extension(),
        ),
    };

    fs::create_dir_all(&dir).map_err(|e| CliError::Io(e))?;
    let path = dir.join(format!("{}.{}", object.envelope.id.as_str(), ext));
    fs::write(path, &object.payload.bytes)?;
    Ok(())
}
