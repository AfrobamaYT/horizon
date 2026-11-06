//! The `displays` area: host methods the game implements (`game/host/displays.rs`).
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `displays` area, implemented by the game and reached through [`ServerHost`].
pub trait DisplaysServerHost {
    /// Side: server.
    /// Narrows an entity to a display, or None for another kind, a stale
    /// handle or a client.
    fn display(&self, entity: EntityRef) -> Option<DisplayRef> {
        crate::phase::unprovided(
            "ServerHost::display",
            "display",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `displays` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait DisplaysBackend {
    /// Side: server.
    /// Which `Display` subclass the entity is.
    fn display_kind(&self, display: DisplayRef) -> Option<NativeDisplayKind> {
        crate::phase::unprovided(
            "Backend::display_kind",
            "display_kind",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.createTransformation`: the transformation as shown.
    fn display_transformation(&self, display: DisplayRef) -> Option<NativeTransformation> {
        crate::phase::unprovided(
            "Backend::display_transformation",
            "display_transformation",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.getBillboardConstraints`.
    fn display_billboard(&self, display: DisplayRef) -> Option<NativeBillboard> {
        crate::phase::unprovided(
            "Backend::display_billboard",
            "display_billboard",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.getBrightnessOverride`: block and sky light, 0 to 15 each, or None for no override.
    fn display_brightness(&self, display: DisplayRef) -> Option<(u8, u8)> {
        crate::phase::unprovided(
            "Backend::display_brightness",
            "display_brightness",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.getViewRange`.
    fn display_view_range(&self, display: DisplayRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::display_view_range",
            "display_view_range",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.getShadowRadius`.
    fn display_shadow_radius(&self, display: DisplayRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::display_shadow_radius",
            "display_shadow_radius",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.getShadowStrength`.
    fn display_shadow_strength(&self, display: DisplayRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::display_shadow_strength",
            "display_shadow_strength",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.getWidth`: the culling box's width, 0 for none.
    fn display_width(&self, display: DisplayRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::display_width",
            "display_width",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.getHeight`: the culling box's height, 0 for none.
    fn display_height(&self, display: DisplayRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::display_height",
            "display_height",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.getGlowColorOverride`: an ARGB color, or -1 for none.
    fn display_glow_color_override(&self, display: DisplayRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::display_glow_color_override",
            "display_glow_color_override",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.getTransformationInterpolationDuration`, in ticks.
    fn display_interpolation_duration(&self, display: DisplayRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::display_interpolation_duration",
            "display_interpolation_duration",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.getTransformationInterpolationDelay`: ticks before an
    /// interpolation starts.
    fn display_start_interpolation(&self, display: DisplayRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::display_start_interpolation",
            "display_start_interpolation",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.getPosRotInterpolationDuration`: ticks a move eases over, 0 to 59.
    fn display_teleport_duration(&self, display: DisplayRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::display_teleport_duration",
            "display_teleport_duration",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TextDisplay.getText`; None for another kind of display.
    fn display_text(&self, display: DisplayRef) -> Option<Text> {
        crate::phase::unprovided(
            "Backend::display_text",
            "display_text",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TextDisplay.getLineWidth`.
    fn display_line_width(&self, display: DisplayRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::display_line_width",
            "display_line_width",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TextDisplay.getBackgroundColor`: ARGB.
    fn display_background(&self, display: DisplayRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::display_background",
            "display_background",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TextDisplay.getTextOpacity`: -1 is opaque, else 0 to 127 then -128 to -2.
    fn display_text_opacity(&self, display: DisplayRef) -> Option<i8> {
        crate::phase::unprovided(
            "Backend::display_text_opacity",
            "display_text_opacity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TextDisplay.FLAG_SHADOW`.
    fn display_text_shadow(&self, display: DisplayRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::display_text_shadow",
            "display_text_shadow",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TextDisplay.FLAG_SEE_THROUGH`.
    fn display_see_through(&self, display: DisplayRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::display_see_through",
            "display_see_through",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TextDisplay.FLAG_USE_DEFAULT_BACKGROUND`.
    fn display_default_background(&self, display: DisplayRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::display_default_background",
            "display_default_background",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TextDisplay.getAlign`, from the left and right flags.
    fn display_alignment(&self, display: DisplayRef) -> Option<NativeTextAlignment> {
        crate::phase::unprovided(
            "Backend::display_alignment",
            "display_alignment",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `BlockDisplay.getBlockState`; None for another kind of display.
    fn display_block_state(&self, display: DisplayRef) -> Option<BlockStateId> {
        crate::phase::unprovided(
            "Backend::display_block_state",
            "display_block_state",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemDisplay.getItemStack`; None for another kind of display or an empty stack.
    fn display_item(&self, display: DisplayRef) -> Option<ItemStack> {
        crate::phase::unprovided(
            "Backend::display_item",
            "display_item",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemDisplay.getItemTransform`: how the item is displayed.
    fn display_item_context(&self, display: DisplayRef) -> Option<NativeItemDisplayContext> {
        crate::phase::unprovided(
            "Backend::display_item_context",
            "display_item_context",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' writes of the `displays` area, implemented by the game and reached through [`BackendMut`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait DisplaysBackendMut {
    /// Side: server.
    /// Sets the transformation. False for a value that isn't finite or a
    /// zero quaternion. Clients ease to it when the start delay is set
    /// (`set_start_interpolation`).
    fn display_set_transformation(
        &mut self,
        display: DisplayRef,
        transformation: NativeTransformation,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_transformation",
            "display_set_transformation",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.setBillboardConstraints`.
    fn display_set_billboard(&mut self, display: DisplayRef, billboard: NativeBillboard) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_billboard",
            "display_set_billboard",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.setBrightnessOverride`; None clears the override. False for a
    /// light level above 15.
    fn display_set_brightness(
        &mut self,
        display: DisplayRef,
        brightness: Option<(u8, u8)>,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_brightness",
            "display_set_brightness",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.setViewRange`. False for a value that isn't finite.
    fn display_set_view_range(&mut self, display: DisplayRef, range: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_view_range",
            "display_set_view_range",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.setShadowRadius`. False for a value that isn't finite.
    fn display_set_shadow_radius(&mut self, display: DisplayRef, radius: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_shadow_radius",
            "display_set_shadow_radius",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.setShadowStrength`. False for a value that isn't finite.
    fn display_set_shadow_strength(&mut self, display: DisplayRef, strength: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_shadow_strength",
            "display_set_shadow_strength",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.setWidth`. False for a value that isn't finite.
    fn display_set_width(&mut self, display: DisplayRef, width: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_width",
            "display_set_width",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.setHeight`. False for a value that isn't finite.
    fn display_set_height(&mut self, display: DisplayRef, height: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_height",
            "display_set_height",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.setGlowColorOverride`.
    fn display_set_glow_color_override(&mut self, display: DisplayRef, color: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_glow_color_override",
            "display_set_glow_color_override",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.setTransformationInterpolationDuration`.
    fn display_set_interpolation_duration(&mut self, display: DisplayRef, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_interpolation_duration",
            "display_set_interpolation_duration",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.setTransformationInterpolationDelay`. Like vanilla, every
    /// call is sent to the clients that track the display, even with the
    /// delay it already has, and each client restarts its interpolation.
    fn display_set_start_interpolation(&mut self, display: DisplayRef, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_start_interpolation",
            "display_set_start_interpolation",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Display.setPosRotInterpolationDuration`, kept from 0 to 59 as
    /// vanilla's load keeps it.
    fn display_set_teleport_duration(&mut self, display: DisplayRef, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_teleport_duration",
            "display_set_teleport_duration",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TextDisplay.setText`; false for another kind of display.
    fn display_set_text(&mut self, display: DisplayRef, text: Text) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_text",
            "display_set_text",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TextDisplay.setLineWidth`; false for another kind of display.
    fn display_set_line_width(&mut self, display: DisplayRef, width: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_line_width",
            "display_set_line_width",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TextDisplay.setBackgroundColor`; false for another kind of display.
    fn display_set_background(&mut self, display: DisplayRef, argb: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_background",
            "display_set_background",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `TextDisplay.setTextOpacity`; false for another kind of display.
    fn display_set_text_opacity(&mut self, display: DisplayRef, opacity: i8) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_text_opacity",
            "display_set_text_opacity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the shadow flag; false for another kind of display.
    fn display_set_text_shadow(&mut self, display: DisplayRef, on: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_text_shadow",
            "display_set_text_shadow",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the see-through flag; false for another kind of display.
    fn display_set_see_through(&mut self, display: DisplayRef, on: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_see_through",
            "display_set_see_through",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the default-background flag; false for another kind of display.
    fn display_set_default_background(&mut self, display: DisplayRef, on: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_default_background",
            "display_set_default_background",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the alignment; false for another kind of display.
    fn display_set_alignment(
        &mut self,
        display: DisplayRef,
        alignment: NativeTextAlignment,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_alignment",
            "display_set_alignment",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `BlockDisplay.setBlockState`; false for another kind of display or an unknown state.
    fn display_set_block_state(&mut self, display: DisplayRef, state: BlockStateId) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_block_state",
            "display_set_block_state",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemDisplay.setItemStack`; None empties it. False for another kind of display.
    fn display_set_item(&mut self, display: DisplayRef, stack: Option<ItemStack>) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_item",
            "display_set_item",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemDisplay.setItemTransform`; false for another kind of display.
    fn display_set_item_context(
        &mut self,
        display: DisplayRef,
        context: NativeItemDisplayContext,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::display_set_item_context",
            "display_set_item_context",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
