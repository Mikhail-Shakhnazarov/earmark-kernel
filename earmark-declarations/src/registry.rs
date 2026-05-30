/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use crate::errors::DeclarationError;
use crate::traits::DeclarationRegistry;
use earmark_core::{
    ClassDeclaration, ClassId, PacketTemplateDeclaration, PacketTemplateId, RelationRule,
    RelationRuleId, RuntimeProtocol, RuntimeProtocolId, SelectionPolicy, SelectionPolicyId,
    SystemDeclaration, SystemId, SystemPackId, SystemPackManifest, ValidatorDeclaration,
    ValidatorId, WorkerProfile, WorkerProfileId, WorkflowDeclaration, WorkflowId,
};
use std::collections::HashMap;

pub struct InProcessRegistry {
    classes: HashMap<ClassId, ClassDeclaration>,
    systems: HashMap<SystemId, SystemDeclaration>,
    workflows: HashMap<WorkflowId, WorkflowDeclaration>,
    packet_templates: HashMap<PacketTemplateId, PacketTemplateDeclaration>,
    protocols: HashMap<RuntimeProtocolId, RuntimeProtocol>,
    validators: HashMap<ValidatorId, ValidatorDeclaration>,
    relation_rules: HashMap<RelationRuleId, RelationRule>,
    selection_policies: HashMap<SelectionPolicyId, SelectionPolicy>,
    worker_profiles: HashMap<WorkerProfileId, WorkerProfile>,
    system_packs: HashMap<SystemPackId, SystemPackManifest>,
}

impl Default for InProcessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl InProcessRegistry {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            systems: HashMap::new(),
            workflows: HashMap::new(),
            packet_templates: HashMap::new(),
            protocols: HashMap::new(),
            validators: HashMap::new(),
            relation_rules: HashMap::new(),
            selection_policies: HashMap::new(),
            worker_profiles: HashMap::new(),
            system_packs: HashMap::new(),
        }
    }

    pub fn register_class(&mut self, decl: ClassDeclaration) {
        self.classes.insert(decl.class_id.clone(), decl);
    }

    pub fn register_system(&mut self, decl: SystemDeclaration) {
        self.systems.insert(decl.system_id.clone(), decl);
    }

    pub fn register_workflow(&mut self, decl: WorkflowDeclaration) {
        self.workflows.insert(decl.workflow_id.clone(), decl);
    }

    pub fn register_packet_template(&mut self, decl: PacketTemplateDeclaration) {
        self.packet_templates
            .insert(decl.packet_template_id.clone(), decl);
    }

    pub fn register_protocol(&mut self, decl: RuntimeProtocol) {
        self.protocols.insert(decl.protocol_id.clone(), decl);
    }

    pub fn register_validator(&mut self, decl: ValidatorDeclaration) {
        self.validators.insert(decl.validator_id.clone(), decl);
    }

    pub fn register_relation_rule(&mut self, decl: RelationRule) {
        self.relation_rules.insert(decl.rule_id.clone(), decl);
    }

    pub fn register_selection_policy(&mut self, decl: SelectionPolicy) {
        self.selection_policies
            .insert(decl.selection_id.clone(), decl);
    }
    pub fn register_worker_profile(&mut self, decl: WorkerProfile) {
        self.worker_profiles
            .insert(decl.worker_profile_id.clone(), decl);
    }
    pub fn register_system_pack(&mut self, decl: SystemPackManifest) {
        self.system_packs.insert(decl.pack_id.clone(), decl);
    }
}

impl DeclarationRegistry for InProcessRegistry {
    fn get_class(&self, id: &ClassId) -> Result<ClassDeclaration, DeclarationError> {
        self.classes
            .get(id)
            .cloned()
            .ok_or_else(|| DeclarationError::NotFound(id.as_str().to_string()))
    }

    fn get_system(&self, id: &SystemId) -> Result<SystemDeclaration, DeclarationError> {
        self.systems
            .get(id)
            .cloned()
            .ok_or_else(|| DeclarationError::NotFound(id.as_str().to_string()))
    }

    fn get_workflow(&self, id: &WorkflowId) -> Result<WorkflowDeclaration, DeclarationError> {
        self.workflows
            .get(id)
            .cloned()
            .ok_or_else(|| DeclarationError::NotFound(id.as_str().to_string()))
    }

    fn get_packet_template(
        &self,
        id: &PacketTemplateId,
    ) -> Result<PacketTemplateDeclaration, DeclarationError> {
        self.packet_templates
            .get(id)
            .cloned()
            .ok_or_else(|| DeclarationError::NotFound(id.as_str().to_string()))
    }

    fn get_runtime_protocol(
        &self,
        id: &RuntimeProtocolId,
    ) -> Result<RuntimeProtocol, DeclarationError> {
        self.protocols
            .get(id)
            .cloned()
            .ok_or_else(|| DeclarationError::NotFound(id.as_str().to_string()))
    }

    fn get_validator(&self, id: &ValidatorId) -> Result<ValidatorDeclaration, DeclarationError> {
        self.validators
            .get(id)
            .cloned()
            .ok_or_else(|| DeclarationError::NotFound(id.as_str().to_string()))
    }

    fn get_relation_rule(&self, id: &RelationRuleId) -> Result<RelationRule, DeclarationError> {
        self.relation_rules
            .get(id)
            .cloned()
            .ok_or_else(|| DeclarationError::NotFound(id.as_str().to_string()))
    }

    fn get_selection_policy(
        &self,
        id: &SelectionPolicyId,
    ) -> Result<earmark_core::SelectionPolicy, DeclarationError> {
        self.selection_policies
            .get(id)
            .cloned()
            .ok_or_else(|| DeclarationError::NotFound(id.as_str().to_string()))
    }

    fn get_worker_profile(&self, id: &WorkerProfileId) -> Result<WorkerProfile, DeclarationError> {
        self.worker_profiles
            .get(id)
            .cloned()
            .ok_or_else(|| DeclarationError::NotFound(id.as_str().to_string()))
    }

    fn get_system_pack(&self, id: &SystemPackId) -> Result<SystemPackManifest, DeclarationError> {
        self.system_packs
            .get(id)
            .cloned()
            .ok_or_else(|| DeclarationError::NotFound(id.as_str().to_string()))
    }

    fn verify_registry(&self) -> Result<Vec<String>, DeclarationError> {
        let mut warnings = vec![];

        for (id, decl) in &self.classes {
            if decl.origin_pack_id.is_none() {
                warnings.push(format!("ClassDeclaration {} missing origin_pack_id", id));
            }
        }
        for (id, decl) in &self.systems {
            if decl.origin_pack_id.is_none() {
                warnings.push(format!("SystemDeclaration {} missing origin_pack_id", id));
            }
        }
        for (id, decl) in &self.workflows {
            if decl.origin_pack_id.is_none() {
                warnings.push(format!("WorkflowDeclaration {} missing origin_pack_id", id));
            }
        }
        for (id, decl) in &self.packet_templates {
            if decl.origin_pack_id.is_none() {
                warnings.push(format!(
                    "PacketTemplateDeclaration {} missing origin_pack_id",
                    id
                ));
            }
        }
        for (id, decl) in &self.validators {
            if decl.origin_pack_id.is_none() {
                warnings.push(format!(
                    "ValidatorDeclaration {} missing origin_pack_id",
                    id
                ));
            }
        }
        for (id, decl) in &self.selection_policies {
            if decl.origin_pack_id.is_none() {
                warnings.push(format!("SelectionPolicy {} missing origin_pack_id", id));
            }
        }

        Ok(warnings)
    }
}
