//! The `scenes_in_screens` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    /// Draws `blocks`, a scene of block states at positions of its own (not
    /// the world's), into the box `rect` (x0, y0, x1, y1, GUI pixels) of a
    /// screen's `batch`, seen as `view` says: Create's Ponder scenes. Each
    /// block is drawn with its model, full bright and shaded by face, a face
    /// against a solid neighbour in the scene left out; a block with an
    /// `entity` (a mod's) is drawn by `native_block_entity_submit` as in the
    /// world too, its `spin` parts turning with the game time. What lies
    /// outside the box is cut off, and the scene has a depth of its own, so
    /// it never covers the screen's other drawing or is covered by it but in
    /// the order drawn. Draw it from a screen's drawing hook
    /// (`native_screen_draw`). A client's call; nothing is drawn on a server.
    /// API @API:scenes_in_screens@.
    ///
    /// Blocks carry API-owned `SceneBlockEntity` drawing snapshots (type and public
    /// fields), not live world entities or renderer-cache identities. The native
    /// renderer uses the same block-entity Part submission and pose/spin path.
    #[helper(
        side = "client",
        target = "horizon_client::game::native_scene::Game<'_>::native_draw_scene"
    )]
    fn draw_scene();
}
