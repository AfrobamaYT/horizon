//! The `model_layers` area: host methods the game implements (`game/host/model_layers.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `model_layers` area, implemented by the game and reached through [`ClientHost`].
pub trait ModelLayersClientHost {
    /// Side: client.
    /// Entity model layer `layer` baked into its `ModelPart` tree, each part
    /// at its pose, as `EntityModelSet.bakeLayer` bakes it: one a native mod
    /// registered (`native_layer_definitions`), or vanilla's, by the name
    /// `entity-models.json` gives it (`minecraft:zombie#main`,
    /// `minecraft:player#main`). The tree is the caller's own copy: bake it
    /// once, as a renderer does, and set its parts' poses each frame. None
    /// for a layer neither has. A vanilla layer no mob has (a player's, a
    /// chest's) is read from the resources each call, so slow. API
    /// @API:model_layers@.
    fn bake_layer(&self, layer: &str) -> Option<ModelPart> {
        crate::phase::unprovided(
            "ClientHost::bake_layer",
            "bake_layer",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
