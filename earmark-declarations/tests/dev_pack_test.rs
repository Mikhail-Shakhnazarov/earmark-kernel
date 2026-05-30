use earmark_core::{ClassId, PacketTemplateId, SystemId, WorkflowId};
use earmark_declarations::{dev_pack, DeclarationRegistry, InProcessRegistry};

/// Protects the integrity of the development-domain declarations.
/// This ensures that the system always boots with the correct classes, workflows, and
/// packet templates required for self-hosting work.
#[test]
fn test_dev_pack_registration() {
    let mut registry = InProcessRegistry::new();
    dev_pack::register_dev_pack(&mut registry);

    // Verify classes
    // Verify classes
    let cls_epic = registry
        .get_class(&ClassId::parse("cls_epic").unwrap())
        .expect("cls_epic not found");
    assert_eq!(cls_epic.title, "Epic");
    registry
        .get_class(&ClassId::parse("cls_decision").unwrap())
        .expect("cls_decision not found");
    let cls_work_item = registry
        .get_class(&ClassId::parse("cls_work_item").unwrap())
        .expect("cls_work_item not found");
    assert_eq!(cls_work_item.title, "Development Work Item");

    let cls_plan = registry
        .get_class(&ClassId::parse("cls_plan").unwrap())
        .expect("cls_plan not found");
    assert_eq!(cls_plan.title, "Implementation Plan");

    let cls_review = registry
        .get_class(&ClassId::parse("cls_review").unwrap())
        .expect("cls_review not found");
    assert_eq!(cls_review.title, "Review Record");

    let cls_artifact = registry
        .get_class(&ClassId::parse("cls_artifact_binding").unwrap())
        .expect("cls_artifact_binding not found");
    assert_eq!(cls_artifact.title, "Artifact Binding");

    // Verify workflow
    let wf = registry
        .get_workflow(&WorkflowId::parse("wf_orchestrate_work").unwrap())
        .expect("wf_orchestrate_work not found");
    assert_eq!(wf.title, "Orchestrate Work");
    assert_eq!(wf.transitions.len(), 7);

    // Verify transitions
    assert_eq!(wf.transitions[0].transition_id, "tr_execute_instruction");
    assert_eq!(wf.transitions[1].transition_id, "tr_propose_plan");
    assert_eq!(wf.transitions[2].transition_id, "tr_record_execution");
    assert_eq!(wf.transitions[3].transition_id, "tr_review_work");
    assert_eq!(wf.transitions[4].transition_id, "tr_report_blocker");
    assert_eq!(wf.transitions[5].transition_id, "tr_generate_evidence");
    assert_eq!(wf.transitions[6].transition_id, "tr_audit_readiness");

    // Verify packet templates
    registry
        .get_packet_template(&PacketTemplateId::parse("pt_dev_planning_packet").unwrap())
        .expect("pt_dev_planning_packet not found");
    registry
        .get_packet_template(&PacketTemplateId::parse("pt_dev_execution_packet").unwrap())
        .expect("pt_dev_execution_packet not found");
    registry
        .get_packet_template(&PacketTemplateId::parse("pt_review_packet").unwrap())
        .expect("pt_review_packet not found");

    registry
        .get_class(&ClassId::parse("cls_handoff").unwrap())
        .expect("cls_handoff not found");

    // Verify system
    let sys = registry
        .get_system(&SystemId::parse("sys_development").unwrap())
        .expect("sys_development not found");
    assert_eq!(sys.title, "Development System");
    assert_eq!(sys.classes.len(), 18);
    assert_eq!(sys.workflows.len(), 1);
}
