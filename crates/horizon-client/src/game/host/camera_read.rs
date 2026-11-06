//! The game's side of the `camera_read` area (`horizon-mod-api/src/host/camera_read.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl CameraReadHost for Game<'_> {
    fn block_collision(&self, pos: [i32; 3]) -> Vec<[f64; 6]> {
        mod_api::__private::check_access(
            "block_collision",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_collision(pos)
    }
}
impl CameraReadClientHost for Game<'_> {
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
}
