//! The game's side of the `display_refs` area (`horizon-mod-api/src/host/display_refs.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl DisplayRefsServerHost for Game<'_> {
    fn display(&self, entity: EntityRef) -> Option<DisplayRef> {
        mod_api::__private::check_access("display", mod_api::__private::LoadAccess::WorldOnly);
        self.native_display(entity)
    }
}
impl DisplayRefsBackend for Game<'_> {
    fn display_kind(&self, display: DisplayRef) -> Option<NativeDisplayKind> {
        mod_api::__private::check_access("display_kind", mod_api::__private::LoadAccess::WorldOnly);
        self.native_display_kind(display)
    }
    fn display_transformation(&self, display: DisplayRef) -> Option<NativeTransformation> {
        mod_api::__private::check_access(
            "display_transformation",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_transformation(display)
    }
    fn display_billboard(&self, display: DisplayRef) -> Option<NativeBillboard> {
        mod_api::__private::check_access(
            "display_billboard",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_billboard(display)
    }
    fn display_brightness(&self, display: DisplayRef) -> Option<(u8, u8)> {
        mod_api::__private::check_access(
            "display_brightness",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_brightness(display)
    }
    fn display_view_range(&self, display: DisplayRef) -> Option<f32> {
        mod_api::__private::check_access(
            "display_view_range",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_view_range(display)
    }
    fn display_shadow_radius(&self, display: DisplayRef) -> Option<f32> {
        mod_api::__private::check_access(
            "display_shadow_radius",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_shadow_radius(display)
    }
    fn display_shadow_strength(&self, display: DisplayRef) -> Option<f32> {
        mod_api::__private::check_access(
            "display_shadow_strength",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_shadow_strength(display)
    }
    fn display_width(&self, display: DisplayRef) -> Option<f32> {
        mod_api::__private::check_access(
            "display_width",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_width(display)
    }
    fn display_height(&self, display: DisplayRef) -> Option<f32> {
        mod_api::__private::check_access(
            "display_height",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_height(display)
    }
    fn display_glow_color_override(&self, display: DisplayRef) -> Option<i32> {
        mod_api::__private::check_access(
            "display_glow_color_override",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_glow_color_override(display)
    }
    fn display_interpolation_duration(&self, display: DisplayRef) -> Option<i32> {
        mod_api::__private::check_access(
            "display_interpolation_duration",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_interpolation_duration(display)
    }
    fn display_start_interpolation(&self, display: DisplayRef) -> Option<i32> {
        mod_api::__private::check_access(
            "display_start_interpolation",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_start_interpolation(display)
    }
    fn display_teleport_duration(&self, display: DisplayRef) -> Option<i32> {
        mod_api::__private::check_access(
            "display_teleport_duration",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_teleport_duration(display)
    }
    fn display_text(&self, display: DisplayRef) -> Option<Text> {
        mod_api::__private::check_access("display_text", mod_api::__private::LoadAccess::WorldOnly);
        self.native_display_text(display)
    }
    fn display_line_width(&self, display: DisplayRef) -> Option<i32> {
        mod_api::__private::check_access(
            "display_line_width",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_line_width(display)
    }
    fn display_background(&self, display: DisplayRef) -> Option<i32> {
        mod_api::__private::check_access(
            "display_background",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_background(display)
    }
    fn display_text_opacity(&self, display: DisplayRef) -> Option<i8> {
        mod_api::__private::check_access(
            "display_text_opacity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_text_opacity(display)
    }
    fn display_text_shadow(&self, display: DisplayRef) -> Option<bool> {
        mod_api::__private::check_access(
            "display_text_shadow",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_text_shadow(display)
    }
    fn display_see_through(&self, display: DisplayRef) -> Option<bool> {
        mod_api::__private::check_access(
            "display_see_through",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_see_through(display)
    }
    fn display_default_background(&self, display: DisplayRef) -> Option<bool> {
        mod_api::__private::check_access(
            "display_default_background",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_default_background(display)
    }
    fn display_alignment(&self, display: DisplayRef) -> Option<NativeTextAlignment> {
        mod_api::__private::check_access(
            "display_alignment",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_alignment(display)
    }
    fn display_block_state(&self, display: DisplayRef) -> Option<BlockStateId> {
        mod_api::__private::check_access(
            "display_block_state",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_block_state(display)
    }
    fn display_item(&self, display: DisplayRef) -> Option<ItemStack> {
        mod_api::__private::check_access("display_item", mod_api::__private::LoadAccess::WorldOnly);
        self.native_display_item(display)
    }
    fn display_item_context(&self, display: DisplayRef) -> Option<NativeItemDisplayContext> {
        mod_api::__private::check_access(
            "display_item_context",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_item_context(display)
    }
}
impl DisplayRefsBackendMut for Game<'_> {
    fn display_set_transformation(
        &mut self,
        display: DisplayRef,
        transformation: NativeTransformation,
    ) -> bool {
        mod_api::__private::check_access(
            "display_set_transformation",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_transformation(display, transformation)
    }
    fn display_set_billboard(&mut self, display: DisplayRef, billboard: NativeBillboard) -> bool {
        mod_api::__private::check_access(
            "display_set_billboard",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_billboard(display, billboard)
    }
    fn display_set_brightness(
        &mut self,
        display: DisplayRef,
        brightness: Option<(u8, u8)>,
    ) -> bool {
        mod_api::__private::check_access(
            "display_set_brightness",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_brightness(display, brightness)
    }
    fn display_set_view_range(&mut self, display: DisplayRef, range: f32) -> bool {
        mod_api::__private::check_access(
            "display_set_view_range",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_view_range(display, range)
    }
    fn display_set_shadow_radius(&mut self, display: DisplayRef, radius: f32) -> bool {
        mod_api::__private::check_access(
            "display_set_shadow_radius",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_shadow_radius(display, radius)
    }
    fn display_set_shadow_strength(&mut self, display: DisplayRef, strength: f32) -> bool {
        mod_api::__private::check_access(
            "display_set_shadow_strength",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_shadow_strength(display, strength)
    }
    fn display_set_width(&mut self, display: DisplayRef, width: f32) -> bool {
        mod_api::__private::check_access(
            "display_set_width",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_width(display, width)
    }
    fn display_set_height(&mut self, display: DisplayRef, height: f32) -> bool {
        mod_api::__private::check_access(
            "display_set_height",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_height(display, height)
    }
    fn display_set_glow_color_override(&mut self, display: DisplayRef, color: i32) -> bool {
        mod_api::__private::check_access(
            "display_set_glow_color_override",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_glow_color_override(display, color)
    }
    fn display_set_interpolation_duration(&mut self, display: DisplayRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "display_set_interpolation_duration",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_interpolation_duration(display, ticks)
    }
    fn display_set_start_interpolation(&mut self, display: DisplayRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "display_set_start_interpolation",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_start_interpolation(display, ticks)
    }
    fn display_set_teleport_duration(&mut self, display: DisplayRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "display_set_teleport_duration",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_teleport_duration(display, ticks)
    }
    fn display_set_text(&mut self, display: DisplayRef, text: Text) -> bool {
        mod_api::__private::check_access(
            "display_set_text",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_text(display, text)
    }
    fn display_set_line_width(&mut self, display: DisplayRef, width: i32) -> bool {
        mod_api::__private::check_access(
            "display_set_line_width",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_line_width(display, width)
    }
    fn display_set_background(&mut self, display: DisplayRef, argb: i32) -> bool {
        mod_api::__private::check_access(
            "display_set_background",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_background(display, argb)
    }
    fn display_set_text_opacity(&mut self, display: DisplayRef, opacity: i8) -> bool {
        mod_api::__private::check_access(
            "display_set_text_opacity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_text_opacity(display, opacity)
    }
    fn display_set_text_shadow(&mut self, display: DisplayRef, on: bool) -> bool {
        mod_api::__private::check_access(
            "display_set_text_shadow",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_text_shadow(display, on)
    }
    fn display_set_see_through(&mut self, display: DisplayRef, on: bool) -> bool {
        mod_api::__private::check_access(
            "display_set_see_through",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_see_through(display, on)
    }
    fn display_set_default_background(&mut self, display: DisplayRef, on: bool) -> bool {
        mod_api::__private::check_access(
            "display_set_default_background",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_default_background(display, on)
    }
    fn display_set_alignment(
        &mut self,
        display: DisplayRef,
        alignment: NativeTextAlignment,
    ) -> bool {
        mod_api::__private::check_access(
            "display_set_alignment",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_alignment(display, alignment)
    }
    fn display_set_block_state(&mut self, display: DisplayRef, state: BlockStateId) -> bool {
        mod_api::__private::check_access(
            "display_set_block_state",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_block_state(display, state)
    }
    fn display_set_item(&mut self, display: DisplayRef, stack: Option<ItemStack>) -> bool {
        mod_api::__private::check_access(
            "display_set_item",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_item(display, stack)
    }
    fn display_set_item_context(
        &mut self,
        display: DisplayRef,
        context: NativeItemDisplayContext,
    ) -> bool {
        mod_api::__private::check_access(
            "display_set_item_context",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_display_set_item_context(display, context)
    }
}
