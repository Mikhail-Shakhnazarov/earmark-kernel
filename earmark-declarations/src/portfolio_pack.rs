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

pub fn get_portfolio_pack_declarations() -> Vec<RegisterFn> {
    let sp_portfolio_public_id = earmark_core::SystemPackId::parse("sp_portfolio_public").unwrap();
    let mut registers: Vec<RegisterFn> = Vec::new();

    // 1. Classes
    let cls_portfolio_item = ClassDeclaration {
        class_id: ClassId::parse("cls_portfolio_item").unwrap(),
        namespace: "earmark.portfolio".to_string(),
        version: "0.1.0".to_string(),
        title: "Portfolio Item".to_string(),
        kind: ClassKind::Object,
        required_headers: vec![],
        payload_schema: PayloadSchema::Markdown,
        intrinsic_signal: true,
        origin_pack_id: Some(sp_portfolio_public_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_portfolio_item)));

    let cls_professional_experience = ClassDeclaration {
        class_id: ClassId::parse("cls_professional_experience").unwrap(),
        namespace: "earmark.portfolio".to_string(),
        version: "0.1.0".to_string(),
        title: "Professional Experience".to_string(),
        kind: ClassKind::Object,
        required_headers: vec![],
        payload_schema: PayloadSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "role": { "type": "string" },
                "company": { "type": "string" },
                "period": { "type": "string" },
                "highlights": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["role", "company"]
        })),
        intrinsic_signal: false,
        origin_pack_id: Some(sp_portfolio_public_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_professional_experience)));

    let cls_architectural_proof = ClassDeclaration {
        class_id: ClassId::parse("cls_architectural_proof").unwrap(),
        namespace: "earmark.portfolio".to_string(),
        version: "0.1.0".to_string(),
        title: "Architectural Proof".to_string(),
        kind: ClassKind::Artifact,
        required_headers: vec![],
        payload_schema: PayloadSchema::Markdown,
        intrinsic_signal: true,
        origin_pack_id: Some(sp_portfolio_public_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_class(cls_architectural_proof)));

    // 2. Workflow
    let wf_portfolio_buildup = WorkflowDeclaration {
        workflow_id: WorkflowId::parse("wf_portfolio_buildup").unwrap(),
        version: "0.1.0".to_string(),
        title: "Portfolio Buildup".to_string(),
        description: "Coordinated buildup of the public portfolio surface".to_string(),
        transitions: vec![
            TransitionDeclaration {
                transition_id: "tr_summarize_project".to_string(),
                kind: TransitionKind::Transform,
                input_contracts: vec![ClassId::parse("cls_portfolio_item").unwrap()],
                output_contracts: vec![ClassId::parse("cls_portfolio_item").unwrap()],
                packet_template_ref: PacketTemplateId::parse("pt_portfolio_execution").unwrap(),
                runtime_protocol_ref: RuntimeProtocolId::parse("rp_default").unwrap(),
                validators: vec![],
            },
            TransitionDeclaration {
                transition_id: "tr_verify_proof".to_string(),
                kind: TransitionKind::Transform,
                input_contracts: vec![ClassId::parse("cls_architectural_proof").unwrap()],
                output_contracts: vec![ClassId::parse("cls_architectural_proof").unwrap()],
                packet_template_ref: PacketTemplateId::parse("pt_portfolio_execution").unwrap(),
                runtime_protocol_ref: RuntimeProtocolId::parse("rp_default").unwrap(),
                validators: vec![],
            },
        ],
        origin_pack_id: Some(sp_portfolio_public_id.clone()),
    };
    registers.push(Box::new(move |r| r.register_workflow(wf_portfolio_buildup)));

    // 3. Packet Templates
    registers.push(Box::new({
        let id = sp_portfolio_public_id.clone();
        move |r| {
            r.register_packet_template(PacketTemplateDeclaration {
                packet_template_id: PacketTemplateId::parse("pt_portfolio_execution").unwrap(),
                version: "0.1.0".to_string(),
                title: "Portfolio Execution Template".to_string(),
                relation_traversal_rules: vec![],
                standing_filters: vec![],
                supports_instructions: true,
                origin_pack_id: Some(id),
            })
        }
    }));

    // 4. System Pack
    registers.push(Box::new({
        move |r| {
            r.register_system_pack(earmark_core::SystemPackManifest {
                pack_id: earmark_core::SystemPackId::parse("sp_portfolio_public").unwrap(),
                namespace: "earmark.portfolio".to_string(),
                version: "0.1.0".to_string(),
                title: "Public Portfolio System Pack".to_string(),
                description: "System pack for the public-facing portfolio buildup".to_string(),
                systems: vec![SystemId::parse("sys_portfolio").unwrap()],
                classes: vec![],
            })
        }
    }));

    // 5. System
    registers.push(Box::new({
        let id = sp_portfolio_public_id.clone();
        move |r| {
            r.register_system(SystemDeclaration {
                system_id: SystemId::parse("sys_portfolio").unwrap(),
                namespace: "earmark.portfolio".to_string(),
                version: "0.1.0".to_string(),
                title: "Public Portfolio System".to_string(),
                description: "System for managing the public portfolio surface".to_string(),
                classes: vec![
                    ClassId::parse("cls_portfolio_item").unwrap(),
                    ClassId::parse("cls_professional_experience").unwrap(),
                    ClassId::parse("cls_architectural_proof").unwrap(),
                ],
                workflows: vec![WorkflowId::parse("wf_portfolio_buildup").unwrap()],
                origin_pack_id: Some(id),
            })
        }
    }));

    registers
}

pub fn register_portfolio_pack(registry: &mut crate::registry::InProcessRegistry) {
    for register in get_portfolio_pack_declarations() {
        register(registry);
    }
}
