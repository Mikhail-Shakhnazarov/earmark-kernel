use earmark_connected_context::WorkSurfaceManifest;
use earmark_core::ObjectRef;

pub(crate) struct ExecutionState<'a> {
    pub(crate) active_objects: &'a mut Vec<ObjectRef>,
    pub(crate) emitted_packets: &'a mut Vec<ObjectRef>,
    pub(crate) emitted_objects: &'a mut Vec<ObjectRef>,
    pub(crate) governance_events: &'a mut Vec<ObjectRef>,
    pub(crate) compiled_context: &'a mut Option<WorkSurfaceManifest>,
}
