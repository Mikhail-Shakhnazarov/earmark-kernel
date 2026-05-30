/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use earmark_core::{
    ClassDeclaration, ClassId, ClassKind, PacketTemplateDeclaration, PacketTemplateId,
    PayloadSchema, RuntimeProtocol, RuntimeProtocolId, SelectionPolicy, SelectionPolicyId,
    SystemDeclaration, SystemId, TransitionDeclaration, TransitionKind, WorkflowDeclaration,
    WorkflowId,
};

type RegisterFn = Box<dyn FnOnce(&mut crate::registry::InProcessRegistry)>;

pub fn get_dev_pack_declarations() -> Vec<RegisterFn> {
    let sp_earmark_dev_id = earmark_core::SystemPackId::parse("sp_earmark_dev").unwrap();
    let mut registers: Vec<RegisterFn> = Vec::new();

    // 1. Classes
    let cls_epic = ClassDeclaration {
        class_id: ClassId::parse("cls_epic").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Epic".to_string(),
        kind: ClassKind::Object,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "description": { "type": "string" },
                "priority": { "enum": ["Low", "Medium", "High"] }
            },
            "required": ["title"]
        })),
        intrinsic_signal: false,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_epic)));

    let cls_work_item = ClassDeclaration {
        class_id: ClassId::parse("cls_work_item").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Development Work Item".to_string(),
        kind: ClassKind::Object,
        required_headers: vec![],
        payload_schema: PayloadSchema::Markdown,
        intrinsic_signal: true,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_work_item)));

    let cls_decision = ClassDeclaration {
        class_id: ClassId::parse("cls_decision").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Architectural Decision".to_string(),
        kind: ClassKind::Object,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "context": { "type": "string" },
                "decision": { "type": "string" },
                "consequences": { "type": "string" }
            },
            "required": ["decision"]
        })),
        intrinsic_signal: false,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_decision)));

    let cls_plan = ClassDeclaration {
        class_id: ClassId::parse("cls_plan").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Implementation Plan".to_string(),
        kind: ClassKind::Object,
        required_headers: vec![],
        payload_schema: PayloadSchema::Markdown,
        intrinsic_signal: true,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_plan)));

    let cls_review = ClassDeclaration {
        class_id: ClassId::parse("cls_review").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Review Record".to_string(),
        kind: ClassKind::Governance,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "outcome": { "enum": ["Approved", "Rejected"] },
                "feedback": { "type": "string" }
            },
            "required": ["outcome"]
        })),
        intrinsic_signal: false,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_review)));

    let cls_artifact_binding = ClassDeclaration {
        class_id: ClassId::parse("cls_artifact_binding").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Artifact Binding".to_string(),
        kind: ClassKind::Artifact,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "uri": { "type": "string" },
                "hash": { "type": "string" },
                "kind": { "type": "string" }
            }
        })),
        intrinsic_signal: true,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_artifact_binding)));

    let cls_selection_policy = ClassDeclaration {
        class_id: ClassId::parse("cls_selection_policy").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Selection Policy".to_string(),
        kind: ClassKind::Declaration,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "required_capabilities": { "type": "array", "items": { "type": "string" } },
                "preference_logic": { "type": "string" }
            }
        })),
        intrinsic_signal: false,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_selection_policy)));

    let cls_provider_profile = ClassDeclaration {
        class_id: ClassId::parse("cls_provider_profile").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Provider Profile".to_string(),
        kind: ClassKind::Object,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "provider_type": { "type": "string" },
                "capabilities": { "type": "array", "items": { "type": "string" } }
            }
        })),
        intrinsic_signal: false,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_provider_profile)));

    let cls_handoff = ClassDeclaration {
        class_id: ClassId::parse("cls_handoff").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Handoff Artifact".to_string(),
        kind: ClassKind::Artifact,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "origin_dispatch": { "type": "string" },
                "changed_artifacts": { "type": "array", "items": { "type": "string" } },
                "unresolved_items": { "type": "array", "items": { "type": "string" } },
                "permitted_next_step": { "type": "string" }
            },
            "required": ["origin_dispatch"]
        })),
        intrinsic_signal: true,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_handoff)));

    let cls_report = ClassDeclaration {
        class_id: ClassId::parse("cls_report").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Dispatch Report".to_string(),
        kind: ClassKind::Artifact,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "dispatch_id": { "type": "string" },
                "summary": { "type": "string" },
                "blockers": { "type": "array", "items": { "type": "string" } },
                "decisions": { "type": "array", "items": { "type": "string" } },
                "readiness": { "type": "string" }
            },
            "required": ["dispatch_id", "summary"]
        })),
        intrinsic_signal: false,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_report)));

    let cls_handoff_artifact = ClassDeclaration {
        class_id: ClassId::parse("cls_handoff_artifact").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Handoff Item".to_string(),
        kind: ClassKind::Object,
        required_headers: vec!["artifact_type".to_string(), "source_dispatch".to_string()],
        payload_schema: PayloadSchema::Any,
        intrinsic_signal: false,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_handoff_artifact)));

    let cls_instruction = ClassDeclaration {
        class_id: ClassId::parse("cls_instruction").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Worker Instruction".to_string(),
        kind: ClassKind::Object,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "objective": { "type": "string" },
                "targets": { "type": "string" },
                "rules": { "type": "string" }
            },
            "required": ["objective"]
        })),
        intrinsic_signal: false,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_instruction)));

    let cls_blocker = ClassDeclaration {
        class_id: ClassId::parse("cls_blocker").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Development Blocker".to_string(),
        kind: ClassKind::Object,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "reason": { "type": "string" },
                "severity": { "enum": ["Low", "Medium", "High", "Critical"] }
            },
            "required": ["reason", "severity"]
        })),
        intrinsic_signal: true,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_blocker)));

    let cls_evidence = ClassDeclaration {
        class_id: ClassId::parse("cls_evidence").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Development Evidence".to_string(),
        kind: ClassKind::Artifact,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "method": { "type": "string" },
                "result": { "type": "string" },
                "verification_ref": { "type": "string" }
            }
        })),
        intrinsic_signal: false,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_evidence)));

    let cls_release_readiness = ClassDeclaration {
        class_id: ClassId::parse("cls_release_readiness").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Release Readiness".to_string(),
        kind: ClassKind::Governance,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "version": { "type": "string" },
                "readiness_score": { "type": "number" },
                "auditor_notes": { "type": "string" }
            }
        })),
        intrinsic_signal: false,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_release_readiness)));
    let cls_evidence_pack = ClassDeclaration {
        class_id: ClassId::parse("cls_evidence_pack").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Evidence Pack".to_string(),
        kind: ClassKind::Artifact,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "evidence_ids": { "type": "array", "items": { "type": "string" } },
                "audit_summary": { "type": "string" },
                "compliance_score": { "type": "number" }
            },
            "required": ["evidence_ids"]
        })),
        intrinsic_signal: true,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_evidence_pack)));
    let cls_continuity_session = ClassDeclaration {
        class_id: ClassId::parse("cls_continuity_session").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Continuity Session".to_string(),
        kind: ClassKind::Object,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": { "type": "string" },
                "current_pointer_id": { "type": "string" },
                "shard_ids": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["session_id"]
        })),
        intrinsic_signal: false,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_continuity_session)));

    let cls_context_shard = ClassDeclaration {
        class_id: ClassId::parse("cls_context_shard").unwrap(),
        namespace: "earmark.dev".to_string(),
        version: "0.1.0".to_string(),
        title: "Context Shard".to_string(),
        kind: ClassKind::Artifact,
        required_headers: vec!["shard_type".to_string()],
        payload_schema: PayloadSchema::Any,
        intrinsic_signal: false,
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_context_shard)));

    // 2. Workflow
    let wf_orchestrate_work = WorkflowDeclaration {
        workflow_id: WorkflowId::parse("wf_orchestrate_work").unwrap(),
        version: "0.1.0".to_string(),
        title: "Orchestrate Work".to_string(),
        description: "Standard Earmark development cycle".to_string(),
        transitions: vec![
            TransitionDeclaration {
                transition_id: "tr_execute_instruction".to_string(),
                kind: TransitionKind::Transform,
                input_contracts: vec![ClassId::parse("cls_instruction").unwrap()],
                output_contracts: vec![ClassId::parse("cls_artifact_binding").unwrap()],
                packet_template_ref: PacketTemplateId::parse("pt_dev_execution_packet").unwrap(),
                runtime_protocol_ref: RuntimeProtocolId::parse("rp_default").unwrap(),
                validators: vec![],
            },
            TransitionDeclaration {
                transition_id: "tr_propose_plan".to_string(),
                kind: TransitionKind::Decision,
                input_contracts: vec![ClassId::parse("cls_work_item").unwrap()],
                output_contracts: vec![ClassId::parse("cls_plan").unwrap()],
                packet_template_ref: PacketTemplateId::parse("pt_dev_planning_packet").unwrap(),
                runtime_protocol_ref: RuntimeProtocolId::parse("rp_default").unwrap(),
                validators: vec![],
            },
            TransitionDeclaration {
                transition_id: "tr_record_execution".to_string(),
                kind: TransitionKind::Transform,
                input_contracts: vec![ClassId::parse("cls_plan").unwrap()],
                output_contracts: vec![ClassId::parse("cls_artifact_binding").unwrap()],
                packet_template_ref: PacketTemplateId::parse("pt_dev_execution_packet").unwrap(),
                runtime_protocol_ref: RuntimeProtocolId::parse("rp_default").unwrap(),
                validators: vec![],
            },
            TransitionDeclaration {
                transition_id: "tr_review_work".to_string(),
                kind: TransitionKind::Review,
                input_contracts: vec![ClassId::parse("cls_artifact_binding").unwrap()],
                output_contracts: vec![ClassId::parse("cls_review").unwrap()],
                packet_template_ref: PacketTemplateId::parse("pt_review_packet").unwrap(),
                runtime_protocol_ref: RuntimeProtocolId::parse("rp_default").unwrap(),
                validators: vec![],
            },
            TransitionDeclaration {
                transition_id: "tr_report_blocker".to_string(),
                kind: TransitionKind::Decision,
                input_contracts: vec![ClassId::parse("cls_work_item").unwrap()],
                output_contracts: vec![ClassId::parse("cls_blocker").unwrap()],
                packet_template_ref: PacketTemplateId::parse("pt_dev_planning_packet").unwrap(),
                runtime_protocol_ref: RuntimeProtocolId::parse("rp_default").unwrap(),
                validators: vec![],
            },
            TransitionDeclaration {
                transition_id: "tr_generate_evidence".to_string(),
                kind: TransitionKind::Transform,
                input_contracts: vec![ClassId::parse("cls_artifact_binding").unwrap()],
                output_contracts: vec![ClassId::parse("cls_evidence").unwrap()],
                packet_template_ref: PacketTemplateId::parse("pt_dev_execution_packet").unwrap(),
                runtime_protocol_ref: RuntimeProtocolId::parse("rp_default").unwrap(),
                validators: vec![],
            },
            TransitionDeclaration {
                transition_id: "tr_audit_readiness".to_string(),
                kind: TransitionKind::Review,
                input_contracts: vec![
                    ClassId::parse("cls_review").unwrap(),
                    ClassId::parse("cls_evidence").unwrap(),
                ],
                output_contracts: vec![ClassId::parse("cls_release_readiness").unwrap()],
                packet_template_ref: PacketTemplateId::parse("pt_review_packet").unwrap(),
                runtime_protocol_ref: RuntimeProtocolId::parse("rp_default").unwrap(),
                validators: vec![],
            },
        ],
        origin_pack_id: Some(sp_earmark_dev_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_workflow(wf_orchestrate_work)));

    // 3. Packet Templates
    registers.push(Box::new({
        let id = sp_earmark_dev_id.clone();
        move |r| {
            r.register_packet_template(PacketTemplateDeclaration {
                packet_template_id: PacketTemplateId::parse("pt_dev_planning_packet").unwrap(),
                version: "0.1.0".to_string(),
                title: "Planning Template".to_string(),
                relation_traversal_rules: vec![],
                standing_filters: vec![],
                supports_instructions: false,
                origin_pack_id: Some(id),
            })
        }
    }));
    registers.push(Box::new({
        let id = sp_earmark_dev_id.clone();
        move |r| {
            r.register_packet_template(PacketTemplateDeclaration {
                packet_template_id: PacketTemplateId::parse("pt_dev_execution_packet").unwrap(),
                version: "0.1.0".to_string(),
                title: "Execution Template".to_string(),
                relation_traversal_rules: vec![],
                standing_filters: vec![],
                supports_instructions: true,
                origin_pack_id: Some(id),
            })
        }
    }));
    registers.push(Box::new({
        let id = sp_earmark_dev_id.clone();
        move |r| {
            r.register_packet_template(PacketTemplateDeclaration {
                packet_template_id: PacketTemplateId::parse("pt_review_packet").unwrap(),
                version: "0.1.0".to_string(),
                title: "Review Template".to_string(),
                relation_traversal_rules: vec![],
                standing_filters: vec![],
                supports_instructions: false,
                origin_pack_id: Some(id),
            })
        }
    }));

    // 4. Runtime Protocol
    registers.push(Box::new(move |r| {
        r.register_protocol(RuntimeProtocol {
            protocol_id: RuntimeProtocolId::parse("rp_default").unwrap(),
            title: "Default Protocol".to_string(),
        })
    }));

    // 4b. Selection Policies
    registers.push(Box::new({
        let id = sp_earmark_dev_id.clone();
        move |r| {
            r.register_selection_policy(SelectionPolicy {
                selection_id: SelectionPolicyId::parse("sel_fastest_worker").unwrap(),
                title: "Fastest Worker Policy".to_string(),
                required_capabilities: vec!["compute".to_string()],
                preference_logic: Some("fastest".to_string()),
                fallback_provider_ref: None,
                fallback_worker_ref: None,
                adapter_kind_filter: None,
                origin_pack_id: Some(id),
            })
        }
    }));

    registers.push(Box::new({
        let _id = sp_earmark_dev_id.clone();
        move |r| {
            r.register_system_pack(earmark_core::SystemPackManifest {
                pack_id: earmark_core::SystemPackId::parse("sp_earmark_dev").unwrap(),
                namespace: "earmark.dev".to_string(),
                version: "0.1.0".to_string(),
                title: "Earmark Development Pack".to_string(),
                description: "Default pack for dev verification".to_string(),
                systems: vec![SystemId::parse("sys_development").unwrap()],
                classes: vec![],
            })
        }
    }));

    // 5. Relation Rules
    registers.push(Box::new({
        let _id = sp_earmark_dev_id.clone();
        move |r| {
            r.register_relation_rule(earmark_core::RelationRule {
                rule_id: earmark_core::RelationRuleId::parse("rule_depends_on").unwrap(),
                relation_type: "depends_on".to_string(),
                source_classes: vec![ClassId::parse("cls_work_item").unwrap()],
                target_classes: vec![ClassId::parse("cls_work_item").unwrap()],
            });
            r.register_relation_rule(earmark_core::RelationRule {
                rule_id: earmark_core::RelationRuleId::parse("rule_belongs_to_epic").unwrap(),
                relation_type: "belongs_to_epic".to_string(),
                source_classes: vec![ClassId::parse("cls_work_item").unwrap()],
                target_classes: vec![ClassId::parse("cls_epic").unwrap()],
            });
        }
    }));

    // 6. System
    registers.push(Box::new({
        let id = sp_earmark_dev_id.clone();
        move |r| {
            r.register_system(SystemDeclaration {
                system_id: SystemId::parse("sys_development").unwrap(),
                namespace: "earmark.dev".to_string(),
                version: "0.1.0".to_string(),
                title: "Development System".to_string(),
                description: "Self-hosting development system".to_string(),
                classes: vec![
                    ClassId::parse("cls_epic").unwrap(),
                    ClassId::parse("cls_work_item").unwrap(),
                    ClassId::parse("cls_decision").unwrap(),
                    ClassId::parse("cls_plan").unwrap(),
                    ClassId::parse("cls_review").unwrap(),
                    ClassId::parse("cls_artifact_binding").unwrap(),
                    ClassId::parse("cls_instruction").unwrap(),
                    ClassId::parse("cls_selection_policy").unwrap(),
                    ClassId::parse("cls_provider_profile").unwrap(),
                    ClassId::parse("cls_handoff").unwrap(),
                    ClassId::parse("cls_worker_profile").unwrap(),
                    ClassId::parse("cls_handoff_artifact").unwrap(),
                    ClassId::parse("cls_blocker").unwrap(),
                    ClassId::parse("cls_evidence").unwrap(),
                    ClassId::parse("cls_release_readiness").unwrap(),
                    ClassId::parse("cls_evidence_pack").unwrap(),
                    ClassId::parse("cls_continuity_session").unwrap(),
                    ClassId::parse("cls_context_shard").unwrap(),
                ],
                workflows: vec![WorkflowId::parse("wf_orchestrate_work").unwrap()],
                origin_pack_id: Some(id),
            })
        }
    }));

    registers
}

pub fn register_dev_pack(registry: &mut crate::registry::InProcessRegistry) {
    for register in get_dev_pack_declarations() {
        register(registry);
    }
}
