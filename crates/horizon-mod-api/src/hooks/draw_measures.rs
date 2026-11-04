//! The `draw_measures` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "client",
        target = "horizon_client::game::native_block_entity_draw::Game<'_>::native_draw_model"
    )]
    fn draw_model();
    #[helper(side = "client")]
    fn text_width();
    #[helper(side = "client")]
    fn sprite_size();
    #[helper(
        side = "client",
        target = "horizon_client::game::native_block_entity_draw::Game<'_>::native_draw_text"
    )]
    fn draw_text();
    #[helper(
        side = "client",
        target = "horizon_client::game::native_block_entity_draw::Game<'_>::native_draw_text_width"
    )]
    fn draw_text_width();
}
