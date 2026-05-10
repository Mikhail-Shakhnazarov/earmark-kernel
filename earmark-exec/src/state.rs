use earmark_connected_context::WorkSurfaceManifest;
use earmark_core::ObjectRef;

pub struct ExecutionState<'a> {
    pub active_objects: &'a mut Vec<ObjectRef>,
    pub emitted_packets: &'a mut Vec<ObjectRef>,
    pub emitted_objects: &'a mut Vec<ObjectRef>,
    pub governance_events: &'a mut Vec<ObjectRef>,
    pub compiled_context: &'a mut Option<WorkSurfaceManifest>,
}
