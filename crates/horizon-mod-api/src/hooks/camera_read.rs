//! The `camera_read` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(side = "client")]
    fn camera();
    /// The entity the view follows: the player, or the entity a server attached the camera to
    /// (a spectator following a mob, a cutscene mod). None outside a world. Read it again each
    /// frame; the handle is for this world and goes stale when it ends.
    #[notes = r#"## Camera entity

`ClientHost::camera_entity()` returns an `EntityRef` for the entity the view follows
(`Minecraft.getCameraEntity`): the local player, unless a server attached the camera to another
entity. It is `None` on the title screen and outside a world. Together with `camera()` and
`camera_type()` it lets a minimap or freecam mod tell its own view from a spectated one."#]
    #[helper(side = "client")]
    fn camera_entity();
    #[helper(side = "both")]
    fn block_collision();
}
