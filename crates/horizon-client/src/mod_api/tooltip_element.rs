use crate::items::ItemStack;
use crate::text::Text;

/// One element of an item's tooltip as `render_tooltip_gather_components`
/// hands it over, before any wrapping
/// (`Either<FormattedText, TooltipComponent>`).
#[derive(Clone)]
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
#[derive(Clone)]
pub struct TooltipImage {
    pub stack: ItemStack,
    pub size: [f32; 2],
}
