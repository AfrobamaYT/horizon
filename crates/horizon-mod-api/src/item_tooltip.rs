//! Moved here from the game so mods can name it.
#[allow(unused_imports)]
use crate::*;
use horizon_core::nbt::Tag;
use horizon_items::items::ItemStack;
/// The `minecraft:tooltip_display` component applied by `ItemStack` while it
/// builds a hover tooltip.
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct TooltipDisplay<'a> {
    pub(crate) component: Option<&'a Tag>,
    pub(crate) ominous_banner: bool,
}
impl<'a> TooltipDisplay<'a> {
    pub fn for_stack(stack: &'a ItemStack) -> Self {
        Self {
            component: stack
                .data()
                .and_then(|data| data.other("minecraft:tooltip_display")),
            ominous_banner: stack.data().is_some_and(|data| data.ominous_banner),
        }
    }

    fn hides_tooltip(&self) -> bool {
        matches!(
            self.component.and_then(|display| display.get("hide_tooltip")),
            Some(Tag::Byte(value)) if *value != 0
        )
    }

    /// `ItemStack.getTooltipLines`: `hide_tooltip` suppresses the name unless
    /// the tooltip flag is creative.
    pub fn name_visible(&self, creative: bool) -> bool {
        creative || !self.hides_tooltip()
    }

    /// `TooltipDisplay.shows`: whether details for a data component are shown.
    pub fn shows(&self, component: &str) -> bool {
        if self.hides_tooltip() {
            return false;
        }
        // `item_data::decode_components` models the raid captain banner's
        // default hidden pattern tooltip as `ominous_banner` instead of
        // retaining its redundant `tooltip_display` component.
        if self.ominous_banner
            && self.component.is_none()
            && component == "minecraft:banner_patterns"
        {
            return false;
        }
        let Some(Tag::List(hidden)) = self
            .component
            .and_then(|display| display.get("hidden_components"))
        else {
            return true;
        };
        !hidden.iter().any(|tag| {
            matches!(tag, Tag::String(id) if id.0.iter().copied().eq(component.encode_utf16()))
        })
    }
}
