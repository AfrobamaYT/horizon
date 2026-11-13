//! A stack tooltip's attribute lines (`ItemStack.addAttributeTooltips`, NeoForge's
//! `AttributeUtil.addAttributeTooltips`): the stack's modifiers read with the player's bases and
//! each attribute's declared style, then the lines mods add.
use super::Game;
use horizon_client::item_tooltip::{
    AttributeStyle, AttributeTooltipContext, AttributeTooltipListener, NoAttributeListener,
    attribute_tooltip,
};
use horizon_client::items::ItemStack;
use horizon_client::text::Text;

impl Game<'_> {
    /// The attribute modifier lines of `stack`'s tooltip and the lines mods add after them.
    /// `shown` is whether the stack's `minecraft:tooltip_display` shows the modifiers; a
    /// `nested` stack (a bundle's, a crossbow's projectile) has no player.
    pub(super) fn stack_attribute_tooltip(
        &self,
        stack: &ItemStack,
        shown: bool,
        nested: bool,
        advanced: bool,
        creative: bool,
        mod_callbacks: bool,
    ) -> (Vec<(String, u32)>, Vec<(Text, u32)>) {
        // The item's own modifiers, then its enchantments' for each slot
        // group (`ItemStack.forEachModifier`).
        let mut modifiers =
            horizon_client::item_attributes::attribute_modifiers(stack, self.items()).into_owned();
        modifiers.extend(self.active_enchantment_definitions().modifier_rows(stack));
        // `Player.getAttributeBaseValue`: the attribute's default where the player has none.
        let base = |name: &str, default| {
            self.player
                .as_ref()
                .and_then(|player| {
                    let key = player.attributes.key(name)?;
                    let handle = player.attributes.lookup(Some(&key))?;
                    player
                        .attributes
                        .instance(&handle)
                        .ok()
                        .map(|value| value.base_value())
                })
                .unwrap_or(default)
        };
        let player_base = (!nested).then(|| {
            [
                base("minecraft:attack_damage", 1.0),
                base(
                    "minecraft:attack_speed",
                    horizon_client::attack_strength::BASE_ATTACK_SPEED,
                ),
            ]
        });
        let context = AttributeTooltipContext {
            player_base,
            advanced,
            creative,
        };
        // `AttributeUtil.addAttributeTooltips`: the one builder the recipe search shares,
        // with the mods' skipped modifiers and added lines. A nested stack has no player.
        let listener = crate::game::native_hooks::HookedAttributeTooltips::new(
            self,
            (!nested).then(|| (self.own_uuid(), self.own_entity_id())),
        );
        let listener: &dyn AttributeTooltipListener = if mod_callbacks {
            &listener
        } else {
            &NoAttributeListener
        };
        let catalog = &self.motion.attributes;
        let style = |attribute: &str| {
            let registered = catalog.attribute(catalog.id(attribute)?);
            Some(AttributeStyle {
                tooltip: registered.tooltip.clone(),
                player_base: (!nested)
                    .then(|| base(attribute, registered.definition.default_value())),
            })
        };
        let translate = |k: &str| self.pack.names.display_get(k);
        attribute_tooltip(
            stack, &modifiers, &context, shown, listener, &translate, &style,
        )
    }
}
