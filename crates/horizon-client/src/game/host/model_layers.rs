//! The game's side of the `model_layers` area (`horizon-mod-api/src/host/model_layers.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl ModelLayersClientHost for Game<'_> {
    fn bake_layer(&self, layer: &str) -> Option<ModelPart> {
        mod_api::__private::check_access(
            "bake_layer",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_bake_layer(layer)
    }
}
