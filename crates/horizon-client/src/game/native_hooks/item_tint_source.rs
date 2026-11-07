use super::*;

impl Game<'_> {
    /// Client-side colour of an item tint source a mod registered (`register_item_tint_sources`),
    /// asked for each entry on it each time the item is resolved. `config` is the entry's JSON
    /// object. Set color, ARGB, only for a source your mod owns; left alone, the entry draws white.
    pub(in crate::game) fn native_item_tint_source(
        &self,
        source: &str,
        config: &serde_json::Value,
        stack: &StackView,
        color: &mut u32,
    ) {
        self.fire_item_tint_source(source, config, stack, color);
    }
}
