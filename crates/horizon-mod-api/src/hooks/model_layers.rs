//! Entity model layer registration, baking and drawing.

pub trait Hooks {
    /// Entity model layers built in code (NeoForge's
    /// `EntityRenderersEvent.RegisterLayerDefinitions`): push each one's id,
    /// `<mod id>:<model>#<layer>` as a `ModelLayerLocation` names it
    /// (`mymod:golem#main`), and its `LayerDefinition`, as the mod's
    /// `createBodyLayer()` builds it. `native_bake_layer` bakes one into the
    /// `ModelPart` tree `native_draw_model_part` draws. Asked once at start,
    /// on the client. An id in vanilla's namespace, not of that form, or
    /// registered already is logged and left out. API @API:model_layers@.
    #[hook(
        side = "client",
        contract = "collect",
        out = "layers",
        context = "registration",
        neoforge = "EntityRenderersEvent.RegisterLayerDefinitions"
    )]
    fn layer_definitions();
}

pub trait Helpers {
    /// Entity model layer `layer` baked into its `ModelPart` tree, each part
    /// at its pose, as `EntityModelSet.bakeLayer` bakes it: one a native mod
    /// registered (`native_layer_definitions`), or vanilla's, by the name
    /// `entity-models.json` gives it (`minecraft:zombie#main`,
    /// `minecraft:player#main`). The tree is the caller's own copy: bake it
    /// once, as a renderer does, and set its parts' poses each frame. None
    /// for a layer neither has. A vanilla layer no mob has (a player's, a
    /// chest's) is read from the resources each call, so slow. API
    /// @API:model_layers@.
    #[helper(
        side = "client",
        target = "horizon_client::game::native_block_entity_draw::Game<'_>::native_bake_layer"
    )]
    fn bake_layer();
    /// Draws model part `part` and the parts under it under `draw`'s pose, as
    /// vanilla's `ModelPart.render` does: each part moved to its place (in
    /// texels, 16 to a block), turned about z, then y, then x, and scaled,
    /// its faces textured from `sprite` (`mymod:entity/golem`, a content
    /// mod's `assets/mymod/textures/entity/golem.png`, or vanilla's
    /// `entity/zombie/zombie`) times the 0xRRGGBB `tint`. A part that isn't
    /// `visible` draws nothing, one that skips drawing (`skip_draw`) only its
    /// children. A mob's model is built in the model's own frame (+y down,
    /// its front towards -z, its feet 24 texels down): in `native_mob_submit`,
    /// `draw.scale([1.0, -1.0, -1.0])` then `draw.translate([0.0, -1.501,
    /// 0.0])` first, as `LivingEntityRenderer` does. False for an unknown
    /// sprite. API @API:model_layers@.
    #[helper(
        side = "client",
        target = "horizon_client::game::native_block_entity_draw::Game<'_>::native_draw_model_part"
    )]
    fn draw_model_part();
}
