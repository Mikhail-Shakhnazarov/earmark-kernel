/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use crate::errors::DeclarationError;
use earmark_core::{
    ClassDeclaration, ClassId, PacketTemplateDeclaration, PacketTemplateId, RelationRule,
    RelationRuleId, RuntimeProtocol, RuntimeProtocolId, SelectionPolicyId, SystemDeclaration,
    SystemId, SystemPackId, SystemPackManifest, ValidatorDeclaration, ValidatorId,
    WorkflowDeclaration, WorkflowId,
};

pub trait DeclarationRegistry {
    fn get_class(&self, id: &ClassId) -> Result<ClassDeclaration, DeclarationError>;
    fn get_system(&self, id: &SystemId) -> Result<SystemDeclaration, DeclarationError>;
    fn get_workflow(&self, id: &WorkflowId) -> Result<WorkflowDeclaration, DeclarationError>;
    fn get_packet_template(
        &self,
        id: &PacketTemplateId,
    ) -> Result<PacketTemplateDeclaration, DeclarationError>;
    fn get_runtime_protocol(
        &self,
        id: &RuntimeProtocolId,
    ) -> Result<RuntimeProtocol, DeclarationError>;
    fn get_validator(&self, id: &ValidatorId) -> Result<ValidatorDeclaration, DeclarationError>;
    fn get_relation_rule(&self, id: &RelationRuleId) -> Result<RelationRule, DeclarationError>;
    fn get_selection_policy(
        &self,
        id: &SelectionPolicyId,
    ) -> Result<earmark_core::SelectionPolicy, DeclarationError>;
    fn get_system_pack(&self, id: &SystemPackId) -> Result<SystemPackManifest, DeclarationError>;
    fn verify_registry(&self) -> Result<Vec<String>, DeclarationError>;
}
