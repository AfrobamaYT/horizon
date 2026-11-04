//! The `display_refs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display"
    )]
    fn display();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_kind"
    )]
    fn display_kind();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_transformation"
    )]
    fn display_transformation();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_transformation"
    )]
    fn display_set_transformation();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_billboard"
    )]
    fn display_billboard();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_billboard"
    )]
    fn display_set_billboard();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_brightness"
    )]
    fn display_brightness();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_brightness"
    )]
    fn display_set_brightness();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_view_range"
    )]
    fn display_view_range();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_view_range"
    )]
    fn display_set_view_range();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_shadow_radius"
    )]
    fn display_shadow_radius();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_shadow_radius"
    )]
    fn display_set_shadow_radius();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_shadow_strength"
    )]
    fn display_shadow_strength();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_shadow_strength"
    )]
    fn display_set_shadow_strength();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_width"
    )]
    fn display_width();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_width"
    )]
    fn display_set_width();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_height"
    )]
    fn display_height();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_height"
    )]
    fn display_set_height();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_glow_color_override"
    )]
    fn display_glow_color_override();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_glow_color_override"
    )]
    fn display_set_glow_color_override();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_interpolation_duration"
    )]
    fn display_interpolation_duration();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_interpolation_duration"
    )]
    fn display_set_interpolation_duration();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_start_interpolation"
    )]
    fn display_start_interpolation();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_start_interpolation"
    )]
    fn display_set_start_interpolation();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_teleport_duration"
    )]
    fn display_teleport_duration();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_teleport_duration"
    )]
    fn display_set_teleport_duration();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_text"
    )]
    fn display_text();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_text"
    )]
    fn display_set_text();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_line_width"
    )]
    fn display_line_width();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_line_width"
    )]
    fn display_set_line_width();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_background"
    )]
    fn display_background();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_background"
    )]
    fn display_set_background();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_text_opacity"
    )]
    fn display_text_opacity();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_text_opacity"
    )]
    fn display_set_text_opacity();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_text_shadow"
    )]
    fn display_text_shadow();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_text_shadow"
    )]
    fn display_set_text_shadow();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_see_through"
    )]
    fn display_see_through();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_see_through"
    )]
    fn display_set_see_through();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_default_background"
    )]
    fn display_default_background();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_default_background"
    )]
    fn display_set_default_background();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_alignment"
    )]
    fn display_alignment();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_alignment"
    )]
    fn display_set_alignment();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_block_state"
    )]
    fn display_block_state();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_block_state"
    )]
    fn display_set_block_state();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_item"
    )]
    fn display_item();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_item"
    )]
    fn display_set_item();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_item_context"
    )]
    fn display_item_context();
    #[helper(
        side = "server",
        receiver = "display",
        target = "horizon_client::game::native_display_refs::Game<'_>::native_display_set_item_context"
    )]
    fn display_set_item_context();
}
