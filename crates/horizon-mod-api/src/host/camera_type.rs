//! The `camera_type` area: host methods the game implements (`game/host/camera_type.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `camera_type` area, implemented by the game and reached through [`ClientHost`].
pub trait CameraTypeClientHost {
    /// Side: client.
    /// The window's camera type (`Options.getCameraType`).
    fn camera_type(&self) -> CameraType {
        crate::phase::unprovided(
            "ClientHost::camera_type",
            "camera_type",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// Sets the window's camera type (`Options.setCameraType`) as F5 does, without
    /// cycling: a freelook mod switches to third person and back.
    fn set_camera_type(&mut self, camera_type: CameraType) {
        crate::phase::unprovided(
            "ClientHost::set_camera_type",
            "set_camera_type",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
