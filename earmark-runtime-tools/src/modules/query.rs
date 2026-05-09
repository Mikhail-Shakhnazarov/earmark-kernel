use earmark_declarations::activate_system_definition;
use earmark_index::{ObjectSummary, QueryFilter, ActiveSystemRecord};
use earmark_store::CanonicalStore;
use crate::modules::error::RuntimeToolError;
use crate::modules::surface::RuntimeToolSurface;

impl<'a, S: CanonicalStore> RuntimeToolSurface<'a, S> {
    pub fn query(&self, filter: QueryFilter) -> Result<Vec<ObjectSummary>, RuntimeToolError> {
        Ok(self.index.query_objects(&filter)?)
    }

    pub fn activate_system(
        &self,
        system_id: &str,
    ) -> Result<ActiveSystemRecord, RuntimeToolError> {
        Ok(activate_system_definition(
            self.store, self.index, system_id,
        )?)
    }
}
