//! The `crosshair_target` area: host methods the game implements (`game/host/crosshair_target.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `crosshair_target` area, implemented by the game and reached through [`ClientHost`].
pub trait CrosshairTargetClientHost {
    /// Side: client.
    /// The ordinary block-or-entity pick (Minecraft.hitResult), using the
    /// game's targeting ray, reach and occlusion. None without a client world
    /// or on a miss. Entity hits carry this client level's epoch. Unlike
    /// native_view_hit, this never traces distant terrain.
    fn crosshair_target(&self) -> Option<NativeCrosshairTarget> {
        crate::phase::unprovided(
            "ClientHost::crosshair_target",
            "crosshair_target",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
