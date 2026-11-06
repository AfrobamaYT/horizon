//! The `tags` area: host methods the game implements (`game/host/tags.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `tags` area, implemented by the game and reached through [`Host`].
pub trait TagsHost {
    /// Side: both.
    /// The saved name for an engine-issued unresolved fluid.
    fn unresolved_fluid_id(&self, fluid: UnresolvedFluid) -> String {
        crate::phase::unprovided(
            "Host::unresolved_fluid_id",
            "unresolved_fluid_id",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether the resolved block is in the world's merged block tag. Unknown tags return false;
    /// unavailable authoritative tags or owning-thread block mappings return None.
    fn block_in_tag(&self, block: BlockKey, tag: &str) -> Option<bool> {
        crate::phase::unprovided(
            "Host::block_in_tag",
            "block_in_tag",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The members of a tag in vanilla's order (`HolderSet.Named` iteration: file order, nested
    /// tags in place, first occurrence kept), over the merged tags. Empty for an unknown tag.
    /// None as for `native_block_in_tag`.
    fn tag_members(&self, registry: TagRegistry, tag: &str) -> Option<Vec<String>> {
        crate::phase::unprovided(
            "Host::tag_members",
            "tag_members",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The outline of `state` as `BlockState.getShape` answers it with an empty getter at the
    /// origin (`EmptyBlockGetter.INSTANCE`, `BlockPos.ZERO`): its boxes, each `[min x, y, z,
    /// max x, y, z]` in the block's own space, with no random offset and none of a world's
    /// context (a state that reads its neighbours or block entity answers what its state alone
    /// gives). Empty for a state with no outline, such as air. None for an id no state has.
    fn state_outline(&self, state: BlockStateId) -> Option<Vec<[f64; 6]>> {
        crate::phase::unprovided(
            "Host::state_outline",
            "state_outline",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The fluid state `state` holds (`BlockState.getFluidState`): water for a waterlogged state,
    /// the empty fluid for most. None for an id no state has.
    fn state_fluid_state(&self, state: BlockStateId) -> Option<FluidState> {
        crate::phase::unprovided(
            "Host::state_fluid_state",
            "state_fluid_state",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether `fluid` is in the fluid tag `tag` (`FluidState.is(TagKey)`, `minecraft:water` or
    /// `water`), over the merged fluid tags. An unknown tag is false. None as for
    /// `native_block_in_tag`.
    fn fluid_in_tag(&self, fluid: &str, tag: &str) -> Option<bool> {
        crate::phase::unprovided(
            "Host::fluid_in_tag",
            "fluid_in_tag",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether `fluid` and `other` are the same fluid (`Fluid.isSame`): a source and its
    /// flowing form are, so `minecraft:water` and `minecraft:flowing_water` are the same, water
    /// and lava not. Names are fluid ids, with or without `minecraft:`; false when either
    /// names no fluid.
    fn fluid_is_same(&self, fluid: &str, other: &str) -> bool {
        crate::phase::unprovided(
            "Host::fluid_is_same",
            "fluid_is_same",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The bucket item that picks `fluid` up (`Fluid.getBucket`), `minecraft:water_bucket` for
    /// water and flowing water. None for `minecraft:empty`, whose bucket is air, and for a name
    /// no fluid has.
    fn fluid_bucket(&self, fluid: &str) -> Option<&'static str> {
        crate::phase::unprovided(
            "Host::fluid_bucket",
            "fluid_bucket",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The sound event a bucket plays as it picks `fluid` up (`Fluid.getPickupSound`),
    /// `minecraft:item.bucket.fill_lava` for lava and flowing lava. None for
    /// `minecraft:empty`, which has none, and for a name no fluid has.
    fn fluid_pickup_sound(&self, fluid: &str) -> Option<&'static str> {
        crate::phase::unprovided(
            "Host::fluid_pickup_sound",
            "fluid_pickup_sound",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
