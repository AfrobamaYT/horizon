//! Per-stack crafting remainders.
pub trait Hooks {
    /// NeoForge's `IItemExtension.getCraftingRemainder(ItemStack)`: what `stack`
    /// leaves when consumed by crafting, furnace fuel or brewing. `remainder` arrives
    /// holding the item's data remainder. Replace it, or set it to `None` to
    /// leave nothing. A part that leaves it alone preserves the data remainder.
    /// Recipe-specific book and banner copies are handled by the recipe.
    /// Asked for each consumption on the authoritative server, with a read-only
    /// `Host`. Oversized counts, unknown items and invalid damage are refused by
    /// the stack boundary, retaining the original data remainder. API stacks
    /// cannot have count zero; use `None` to leave nothing. Native panics follow
    /// the normal Part crash path, attributed to the owning mod and Part.
    #[hook(
        side = "server",
        host = "read",
        contract = "listen",
        scope = "item",
        key = "stack",
        neoforge = "IItemExtension.getCraftingRemainder(ItemStack)"
    )]
    fn crafting_remainder();
}
