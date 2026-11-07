//! The game's side of the `client_view` area (`horizon-mod-api/src/host/client_view.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ClientViewHost for Game<'_> {
    fn block_collision(&self, pos: [i32; 3]) -> Vec<[f64; 6]> {
        mod_api::__private::check_access(
            "block_collision",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_collision(pos)
    }
}
impl ClientViewClientHost for Game<'_> {
    fn camera_type(&self) -> CameraType {
        mod_api::__private::check_access(
            "camera_type",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_camera_type()
    }
    fn set_camera_type(&self, camera_type: CameraType) {
        mod_api::__private::check_access(
            "set_camera_type",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_set_camera_type(camera_type)
    }
    fn clock(&self) -> Option<NativeClock> {
        mod_api::__private::check_access("clock", mod_api::__private::LoadAccess::ClientSetup);
        self.native_clock()
    }
    fn raycast(&self, distance: f32) -> Option<NativeHit> {
        mod_api::__private::check_access("raycast", mod_api::__private::LoadAccess::ClientSetup);
        self.native_raycast(distance)
    }
    fn view_hit(&self) -> Option<(f32, NativeHit)> {
        mod_api::__private::check_access("view_hit", mod_api::__private::LoadAccess::ClientSetup);
        self.native_view_hit()
    }
    fn camera(&self) -> Option<NativeCamera> {
        mod_api::__private::check_access("camera", mod_api::__private::LoadAccess::ClientSetup);
        self.native_camera()
    }
    fn camera_entity(&self) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "camera_entity",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_camera_entity()
    }
    fn is_paused(&self) -> bool {
        mod_api::__private::check_access("is_paused", mod_api::__private::LoadAccess::ClientSetup);
        self.native_is_paused()
    }
    fn crosshair_target(&self) -> Option<NativeCrosshairTarget> {
        mod_api::__private::check_access(
            "crosshair_target",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_crosshair_target()
    }
}
