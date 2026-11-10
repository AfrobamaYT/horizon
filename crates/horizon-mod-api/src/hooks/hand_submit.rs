//! The `hand_submit` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Draw in one first-person hand of this window's player, once a frame
    /// for each hand `native_render_hand` is asked about, right after it
    /// (`RenderHandEvent`'s pose stack and submit collector): a hand that
    /// holds an item or a map, the empty main hand's arm, and the empty off
    /// hand, where vanilla draws nothing. `native_render_arm` only covers
    /// vanilla's arms. `render` is what `native_render_hand` got, with the
    /// pose the mods set, and `canceled` says whether they left vanilla's
    /// hand out. `draw`'s pose stack starts in the hands' view space, in
    /// blocks, +x right, +y up, -z into the screen, with the hand sway and
    /// that pose applied and none of vanilla's per-item transforms; both
    /// hands share it, and `render.hand` says which one is asked. It is lit
    /// with the light at the eyes; add geometry with `native_draw_cube`,
    /// `native_draw_quad`, `native_draw_block` and `native_draw_item`. It
    /// joins the hand's own vertices: the hand's field of view, the view
    /// bob, a shader pack's hand pass and `native_hand_drawn` see it as they
    /// see vanilla's hand. To replace a hand, cancel it in
    /// `native_render_hand` and draw here; what the mods drew stays.
    /// Client-side.
    #[notes = r##"## Drawing in a first-person hand

`Hooks::hand_submit` is the drawing half of NeoForge's `RenderHandEvent`: `render_hand` poses or cancels a
first-person hand, `hand_submit` draws in it. It is asked once a frame for each hand `render_hand` is asked
about, right after it, with the same `HandRender` (the pose the mods set included), whether the default
geometry was left in or cancelled, and a draw whose pose stack starts in the hands' view space: blocks, +x
right, +y up, -z into the screen, with the hand sway and the mods' pose applied and none of vanilla's per-item
transforms. Both hands share that space; `render.hand` says which one is asked.

- **Replace a hand**: cancel it in `render_hand` and draw here. What the mods drew stays.
- **Add to a hand**: draw here without cancelling; the default geometry is appended after.
- It covers every hand `render_hand` does: a hand that holds an item, where `render_arm` (vanilla's arm
  only: the empty main hand, the arms under a map) never fires, and the empty off hand, where vanilla draws
  nothing, so a two-handed pose can draw its second arm.
- The geometry joins the hand's own vertices: it gets the hand's field of view and the view bob, a shader
  pack's hand pass draws it, and `hand_drawn` sees it. A hand a mod drew in starts a run of its own with no
  rendered item id, as the arm does.
- With no listener the game builds nothing for it: one `listened` test per visible hand.
"##]
    #[hook(
        side = "client",
        contract = "listen",
        neoforge = "RenderHandEvent",
        cancellable = true
    )]
    fn hand_submit();
}
