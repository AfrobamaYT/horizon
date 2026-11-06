//! The `client_view` area: host methods the game implements (`game/host/client_view.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `client_view` area, implemented by the game and reached through [`Host`].
pub trait ClientViewHost {
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
/// The client host's methods of the `client_view` area, implemented by the game and reached through [`ClientHost`].
pub trait ClientViewClientHost {
    /// Side: client.
    /// The clock of the world this window shows, for client mods: its game
    /// time and time of day as the server last sent them
    /// (`ClientboundSetTimePacket`), advanced each tick between packets; none
    /// on the title screen.
    fn clock(&self) -> Option<NativeClock> {
        crate::phase::unprovided(
            "ClientHost::clock",
            "clock",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The first block along the view within `distance` (at most 1024), past the loaded chunks over the distant terrain.
    fn raycast(&self, distance: f32) -> Option<NativeHit> {
        crate::phase::unprovided(
            "ClientHost::raycast",
            "raycast",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The first block along the view within `RAYCAST_MAX`, and its distance.
    fn view_hit(&self) -> Option<(f32, NativeHit)> {
        crate::phase::unprovided(
            "ClientHost::view_hit",
            "view_hit",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
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
    /// Side: client.
    /// The entity the view follows (`Minecraft.getCameraEntity`): the entity a server attached
    /// the camera to, else the local player. None outside a client world.
    fn camera_entity(&self) -> Option<EntityRef> {
        crate::phase::unprovided(
            "ClientHost::camera_entity",
            "camera_entity",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
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
    /// Side: client.
    /// The applied client pause state, not whether a pause screen is open.
    fn is_paused(&self) -> bool {
        crate::phase::unprovided(
            "ClientHost::is_paused",
            "is_paused",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
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
