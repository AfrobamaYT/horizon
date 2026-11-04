use horizon_client_core::text::Text;
use horizon_items::items::ItemStack;

/// One element of an item's tooltip as `render_tooltip_gather_components`
/// hands it over, before any wrapping
/// (`Either<FormattedText, TooltipComponent>`).
#[derive(Clone, PartialEq)]
// exhaustive: a closed set the game matches without a wildcard
pub enum TooltipElement {
    /// A line (`FormattedText`) with its colour as `rgba` packs it
    /// (`u32::MAX` keeps the text's own).
    Text(Text, u32),
    /// An image component (`TooltipComponent`).
    Image(TooltipImage),
}

/// An image component of a tooltip: `size` GUI pixels, drawn by the
/// `tooltip_component_draw` part of `stack`'s item, with `stack`'s data.
/// An item's own `tooltip_component` part answers the one it starts with.
#[derive(Clone, PartialEq)]
// exhaustive: the game builds it with a literal; mods only read it
pub struct TooltipImage {
    /// The stack.
    pub stack: ItemStack,
    /// The size.
    pub size: [f32; 2],
}
