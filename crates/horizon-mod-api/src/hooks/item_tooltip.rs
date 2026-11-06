//! The `item_tooltip` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `ItemTooltipEvent`: the whole tooltip of `stack` (`ItemStack.getTooltipLines`),
    /// once its name, the item's own lines, the components' lines, the
    /// advanced lines and the operator warning are in `lines`, each with a
    /// colour as `rgba` packs it (`u32::MAX` keeps the text's own). Add,
    /// change, reorder or remove lines. `player` and `player_entity_id` are
    /// the local player's, `display` the stack's `minecraft:tooltip_display`,
    /// `advanced` (F3+H) and `creative` (a creative tab's own slot) its
    /// `TooltipFlag`. Asked each frame the tooltip shows, never for one the
    /// stack hides or a crossbow's loaded projectile. Client-side.
    #[hook(side = "client", contract = "listen", neoforge = "ItemTooltipEvent")]
    fn item_tooltip(
        player: u128,
        player_entity_id: i32,
        stack: &ItemStack,
        display: &TooltipDisplay<'_>,
        advanced: bool,
        creative: bool,
        lines: &mut Vec<(Text, u32)>,
    );
    /// The lines of a data component a mod registered (`register_tooltip_appenders`): added after the
    /// item's own and the components' lines, before the durability and advanced lines, for each registered
    /// component `stack` carries that its `tooltip_display` shows. `component` is the registered id
    /// (`mymod:charge`). Every mod is asked, so add lines only for the components you registered. Lines go
    /// in `lines` as `item_tooltip` gives them. Client-side.
    #[hook(side = "client", contract = "listen")]
    fn tooltip_appender();
}
