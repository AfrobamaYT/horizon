//! The `extract_block_screen_effect` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Before one of the first-person view's block overlays (`ScreenEffectRenderer`) is drawn,
    /// once a frame each is shown: `Block` with the view-blocking `state` at `pos` the camera is
    /// inside, `Water` with the `state` at the eye's block `pos` while the eye is under water, and
    /// `Fire` with fire's default `state` at the player's block `pos` while they burn. Never for a
    /// spectator; `Fire` only in first person. Set `canceled` to leave that overlay out; the
    /// frame, the other block overlays and the HUD's camera overlays are drawn as before.
    /// Client-side.
    #[hook(
        side = "client",
        contract = "cancel",
        out = "canceled",
        neoforge = "ExtractBlockScreenEffectEvent",
        cancellable = true
    )]
    #[notes = r##"## Block screen effects

`native_extract_block_screen_effect(effect, state, pos, canceled)` is NeoForge's `ExtractBlockScreenEffectEvent`: once a frame for each of the first-person view's block overlays that is shown, before its quads are built. `Block` is the view-blocking block the camera is inside, `Water` the block at the eye while it is under water, and `Fire` fire's default state at the player's block while they burn (first person only). None of them fire for a spectator. Set `canceled` to leave that one overlay out; the other overlays and the HUD's camera overlays (vignette, pumpkin, portal, freezing) stay. With no listener the frame pays one `listened` check."##]
    fn extract_block_screen_effect();
}
