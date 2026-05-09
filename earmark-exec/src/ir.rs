use earmark_core::{ObjectRef, VersionRef, RunRecord, ObjectId, StandingConstraint, RequiredCheck};

#[derive(Debug, Clone)]
pub struct ExecutionTransition {
    pub id: String,
    pub operation: String,
    pub input_contracts: Vec<String>,
    pub output_contracts: Vec<String>,
    pub instruction: Option<VersionRef>,
    pub compiled_context: Option<VersionRef>,
    pub policy: Option<VersionRef>,
    pub provider_profile: Option<VersionRef>,
}

#[derive(Debug, Clone)]
pub struct ExecutionEdge {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExecutionIr {
    pub transitions: Vec<ExecutionTransition>,
    pub guards: Vec<earmark_core::WorkflowGuard>,
    pub edges: Vec<ExecutionEdge>,
}

#[derive(Debug, Clone)]
pub struct WorkflowRunRequest {
    pub run_id: String,
    pub system_definition: VersionRef,
    pub workflow: VersionRef,
    pub inputs: Vec<ObjectRef>,
    pub handoff_manifest: Option<earmark_core::HandoffManifestId>,
    pub transition_assignment: Option<earmark_core::TransitionAssignmentId>,
    pub operator_approved: bool,
}

#[derive(Debug, Clone)]
pub struct WorkflowRunOutcome {
    pub record: RunRecord,
    pub emitted_packets: Vec<ObjectRef>,
    pub emitted_objects: Vec<ObjectRef>,
    pub governance_events: Vec<ObjectRef>,
}

#[derive(Debug, Clone, Default)]
pub struct SuccessorHandoffSpec {
    pub to_transition_id: Option<String>,
    pub allowed_input_classes: Vec<String>,
    pub allowed_output_classes: Vec<String>,
    pub allowed_relation_types: Vec<String>,
    pub standing_constraints: Vec<StandingConstraint>,
    pub required_checks: Vec<RequiredCheck>,
    pub compiled_context_template_id: Option<ObjectId>,
}

#[derive(Debug, Clone)]
pub struct TransformArtifacts {
    pub output: ObjectRef,
    pub relation_ids: Vec<ObjectId>,
}
