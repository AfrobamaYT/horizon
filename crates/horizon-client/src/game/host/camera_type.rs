//! The game's side of the `camera_type` area (`horizon-mod-api/src/host/camera_type.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl CameraTypeClientHost for Game<'_> {
    fn camera_type(&self) -> CameraType {
        mod_api::__private::check_access(
            "camera_type",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_camera_type()
    }
    fn set_camera_type(&mut self, camera_type: CameraType) {
        mod_api::__private::check_access(
            "set_camera_type",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_set_camera_type(camera_type)
    }
}
