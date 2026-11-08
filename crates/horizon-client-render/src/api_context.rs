//! Borrowed adapters for load and loop hooks shared by client-lib and gameplay.
use horizon_world::native_load_context as engine;
use std::sync::Arc;

/// Convert the owning load's phase, without reading resources.
pub fn load_origin(origin: engine::DataLoadOrigin) -> mod_api::DataLoadOrigin {
    match origin {
        engine::DataLoadOrigin::ServerDataLoad => mod_api::DataLoadOrigin::ServerDataLoad,
        engine::DataLoadOrigin::ClientPreload => mod_api::DataLoadOrigin::ClientPreload,
        _ => unreachable!("all engine load origins are mapped"),
    }
}

/// Convert the loop's owning side.
pub fn loop_side(side: engine::LoopSide) -> mod_api::LoopSide {
    match side {
        engine::LoopSide::Server => mod_api::LoopSide::Server,
        engine::LoopSide::Client => mod_api::LoopSide::Client,
        _ => unreachable!("all engine loop sides are mapped"),
    }
}

/// Borrows one owning load's frozen metadata, without scanning or copying it.
pub struct LoadReader<'a>(pub &'a engine::LoadContext);
impl LoadReader<'_> {
    /// Build the API view after the hook's subscription guard.
    pub fn context(&self) -> mod_api::OwningLoad<'_> {
        mod_api::OwningLoad::new(load_origin(self.0.origin()), self)
    }
}
impl mod_api::LoadReader for LoadReader<'_> {
    fn registry(&self, name: &str) -> Option<Arc<[String]>> {
        self.0.registry(name)
    }
    fn fluid_tags(&self) -> Arc<[(String, Arc<[String]>)]> {
        self.0.fluid_tags().clone()
    }
    fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        self.0.setting(mod_id, key)
    }
}

/// Borrows one loop's frozen metadata, without scanning or copying it.
pub struct LoopReader<'a>(pub &'a engine::LoopContext);
impl LoopReader<'_> {
    /// Build the API view after the hook's subscription guard.
    pub fn context(&self) -> mod_api::OwningLoop<'_> {
        mod_api::OwningLoop::new(loop_side(self.0.side()), self)
    }
}
impl mod_api::LoopReader for LoopReader<'_> {
    fn registry(&self, name: &str) -> Option<Arc<[String]>> {
        self.0.registry(name)
    }
    fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        self.0.setting(mod_id, key)
    }
}
