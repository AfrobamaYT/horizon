//! Load-bound worker metadata. Each numeric registry retains its own immutable snapshot.
use super::api_values::block_state::{self, Index};
use horizon_mod_runtime::block_definitions::Blocks;
use horizon_world::{registry::BlockRegistry, worldgen::placement::PlacementQueryError};
use mod_api::{BlockKey, BlockMetadata, BlockState};
use std::sync::Arc;

/// No thread-local game, Host, registry rebuild or name resolution is needed by workers.
pub(in crate::game) struct WorkerResources {
    pub index: Arc<Index>,
    pub metadata: &'static Blocks,
    air: BlockState,
    air_blocks: [Option<BlockKey>; 3],
}
impl WorkerResources {
    pub fn get(registry: &BlockRegistry) -> Result<&Self, PlacementQueryError> {
        registry
            .mod_worker_resource
            .get()
            .and_then(|resource| resource.downcast_ref())
            .ok_or(PlacementQueryError::Unavailable)
    }
    pub fn state(&self, block: BlockKey) -> BlockState {
        BlockState::of(self.metadata, block, &[]).unwrap_or(self.air)
    }
    pub fn state_with(
        &self,
        registry: &BlockRegistry,
        block: BlockKey,
        properties: &[(&str, &str)],
    ) -> BlockState {
        // Keep the engine's partial-property/default-state rules, including duplicate properties.
        // This is a cold construction query, not the indexed placed-block conversion path.
        let name = self.metadata.name(block).unwrap_or("minecraft:air");
        let state =
            horizon_world::worldgen::structure::legacy::block_state(registry, name, properties);
        self.index.to_api(state).unwrap_or(self.air)
    }
    pub fn is_air(&self, state: BlockState) -> bool {
        self.index.to_engine(state).is_ok() && self.air_blocks.contains(&Some(state.block()))
    }
}

/// Freeze and return the very same index that owning threads install with PreparedParts.
/// A new or reordered registry has a separate once-initialized resource, never a global index.
pub(in crate::game) fn prepare(registry: &BlockRegistry) -> Result<Arc<Index>, String> {
    retain(registry, block_state::prepare(registry)?)
}

/// Attach the already checked index from the owning-thread resolution boundary.
pub(in crate::game) fn retain(
    registry: &BlockRegistry,
    index: Arc<Index>,
) -> Result<Arc<Index>, String> {
    let metadata = mod_api::__private::block_definitions()
        .ok_or_else(|| "worker block metadata is unavailable".to_owned())?;
    if let Ok(resource) = WorkerResources::get(registry) {
        if !std::ptr::eq(resource.metadata, metadata) {
            return Err(
                "worker block registry belongs to a different metadata snapshot".to_owned(),
            );
        }
        return Ok(resource.index.clone());
    }
    let air = registry
        .default_state("minecraft:air")
        .ok_or_else(|| "worker block registry has no minecraft:air".to_owned())?;
    let air = index
        .to_api(air)
        .map_err(|error| format!("worker air state: {error:?}"))?;
    let air_blocks = ["minecraft:air", "minecraft:cave_air", "minecraft:void_air"].map(|name| {
        registry
            .default_state(name)
            .and_then(|state| index.to_api(state).ok())
            .map(BlockState::block)
    });
    let _ = registry.mod_worker_resource.set(Arc::new(WorkerResources {
        index,
        metadata,
        air,
        air_blocks,
    }));
    WorkerResources::get(registry)
        .map(|resource| resource.index.clone())
        .map_err(|_| "worker registry contains an incompatible resource".to_owned())
}
