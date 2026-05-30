/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use crate::errors::DeclarationError;
use earmark_core::{
    ClassDeclaration, ObjectRecord, PayloadSchema, RelationRecord, RelationRule, VersionRecord,
};

pub fn validate_object_envelope(
    class_decl: &ClassDeclaration,
    _object_record: &ObjectRecord,
    _version_record: &VersionRecord,
) -> Result<(), DeclarationError> {
    // Validate required headers
    for _header in &class_decl.required_headers {
        // Basic check for existence in version_record.payload?
        // Actually records/core.rs VersionRecord doesn't have headers yet, wait.
        // Pseudocode says ClassDeclaration has required_headers.
        // For now, this is a placeholder.
    }

    // Validate payload schema
    match &class_decl.payload_schema {
        PayloadSchema::Any => {}
        PayloadSchema::JsonSchema(_schema) => {
            // TODO: Implement JSON schema validation
        }
        PayloadSchema::Markdown => {
            // Basic check if it's a string?
        }
        PayloadSchema::Text => {}
        PayloadSchema::BinaryRef => {}
    }

    Ok(())
}

pub fn validate_relation_envelope(
    rule: &RelationRule,
    record: &RelationRecord,
) -> Result<(), DeclarationError> {
    // Validate source and target classes
    // (This requires looking up the source/target objects in the store/index,
    // but the validation itself can be pure if class information is provided)

    // Validate relation type matches rule
    if record.relation_type != rule.relation_type {
        return Err(DeclarationError::SchemaInvalid(format!(
            "relation type mismatch: expected {}, got {}",
            rule.relation_type, record.relation_type
        )));
    }

    Ok(())
}
