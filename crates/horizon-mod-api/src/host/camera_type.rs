//! The `camera_type` area: host methods the game implements (`game/host/camera_type.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `camera_type` area, implemented by the game and reached through [`ClientHost`].
pub trait CameraTypeClientHost {
    /// Side: client.
    /// The window's camera type (`Options.getCameraType`), including its latest pending request.
    fn camera_type(&self) -> CameraType {
        crate::phase::unprovided(
            "ClientHost::camera_type",
            "camera_type",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// Request the window's camera type without cycling, also from shared-host callbacks.
    /// The last request wins and [`Self::camera_type`] reports it immediately. Requests before
    /// or during `camera_pose` apply to that frame's camera, FOV and spectator post effects;
    /// later requests apply at the next frame's camera extraction. Explicit pose overrides
    /// remain unchanged when vanilla placement is refreshed. No cycle hook is emitted.
    fn set_camera_type(&self, camera_type: CameraType) {
        crate::phase::unprovided(
            "ClientHost::set_camera_type",
            "set_camera_type",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
