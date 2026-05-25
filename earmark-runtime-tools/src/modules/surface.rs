use earmark_exec::ProviderService;
use earmark_index::DerivedIndex;
use earmark_store::CanonicalStore;

pub struct RuntimeToolSurface<'a, S: CanonicalStore> {
    pub store: &'a S,
    pub index: &'a mut DerivedIndex,
    pub provider_service: &'a dyn ProviderService,
}

impl<'a, S: CanonicalStore> RuntimeToolSurface<'a, S> {
    pub fn new(
        store: &'a S,
        index: &'a mut DerivedIndex,
        provider_service: &'a dyn ProviderService,
    ) -> Self {
        Self {
            store,
            index,
            provider_service,
        }
    }
}
