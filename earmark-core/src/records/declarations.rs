/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use crate::ids::{
    ClassId, PacketTemplateId, ProviderProfileId, RelationRuleId, RuntimeProtocolId,
    SelectionPolicyId, SystemId, SystemPackId, ValidatorId, WorkflowId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemPackManifest {
    pub pack_id: SystemPackId,
    pub namespace: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub systems: Vec<SystemId>,
    pub classes: Vec<ClassId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemDeclaration {
    pub system_id: SystemId,
    pub namespace: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub classes: Vec<ClassId>,
    pub workflows: Vec<WorkflowId>,
    pub origin_pack_id: Option<SystemPackId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassDeclaration {
    pub class_id: ClassId,
    pub namespace: String,
    pub version: String,
    pub title: String,
    pub kind: ClassKind,
    pub required_headers: Vec<String>,
    pub payload_schema: PayloadSchema,
    #[serde(default)]
    pub intrinsic_signal: bool,
    pub origin_pack_id: Option<SystemPackId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassKind {
    Object,
    Artifact,
    Declaration,
    Governance,
    RuntimeRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayloadSchema {
    Any,
    JsonSchema(serde_json::Value),
    Markdown,
    Text,
    BinaryRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDeclaration {
    pub workflow_id: WorkflowId,
    pub version: String,
    pub title: String,
    pub description: String,
    pub transitions: Vec<TransitionDeclaration>,
    pub origin_pack_id: Option<SystemPackId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionDeclaration {
    pub transition_id: String, // TransitionId might be shared across workflows, or we use String for simplicity in decls
    pub kind: TransitionKind,
    pub input_contracts: Vec<ClassId>,
    pub output_contracts: Vec<ClassId>,
    pub packet_template_ref: PacketTemplateId,
    pub runtime_protocol_ref: RuntimeProtocolId,
    pub validators: Vec<ValidatorId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionKind {
    CompilePacket,
    Transform,
    Review,
    Decision,
    Report,
    ExternalAction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PacketTemplateDeclaration {
    pub packet_template_id: PacketTemplateId,
    pub version: String,
    pub title: String,
    pub relation_traversal_rules: Vec<RelationRuleId>,
    pub standing_filters: Vec<StandingFilter>,
    #[serde(default)]
    pub supports_instructions: bool,
    pub origin_pack_id: Option<SystemPackId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingFilter {
    pub dimension: String,
    pub permitted_tokens: Vec<String>,
    pub rejection_rationale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeProtocol {
    pub protocol_id: RuntimeProtocolId,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidatorDeclaration {
    pub validator_id: ValidatorId,
    pub title: String,
    pub validator_type: String,
    pub config: serde_json::Value,
    pub origin_pack_id: Option<SystemPackId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionPolicy {
    pub selection_id: SelectionPolicyId,
    pub title: String,
    pub required_capabilities: Vec<String>,
    pub preference_logic: Option<String>, // e.g., "fastest", "cheapest"
    pub fallback_provider_ref: Option<ProviderProfileId>,
    pub origin_pack_id: Option<SystemPackId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationRule {
    pub rule_id: RelationRuleId,
    pub relation_type: String,
    pub source_classes: Vec<ClassId>,
    pub target_classes: Vec<ClassId>,
}
