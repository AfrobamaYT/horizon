//! The `camera_read` area: host methods the game implements (`game/host/camera_read.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `camera_read` area, implemented by the game and reached through [`Host`].
pub trait CameraReadHost {
    /// Side: both.
    /// Collision boxes in block-local coordinates with an empty collision context.
    fn block_collision(&self, pos: [i32; 3]) -> Vec<[f64; 6]> {
        crate::phase::unprovided(
            "Host::block_collision",
            "block_collision",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The client host's methods of the `camera_read` area, implemented by the game and reached through [`ClientHost`].
pub trait CameraReadClientHost {
    /// Side: client.
    /// The most recently extracted world frame's camera, if one exists.
    /// During GUI rendering this is the current frame, including bob/spin and FOV.
    fn camera(&self) -> Option<NativeCamera> {
        crate::phase::unprovided(
            "ClientHost::camera",
            "camera",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
