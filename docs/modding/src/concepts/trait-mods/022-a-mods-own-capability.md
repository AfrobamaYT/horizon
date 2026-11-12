<!-- chapter: A mod's own capability -->
<!-- separator: 1 -->

#### A mod's own capability

A mod can define a capability of its own, as Mekanism defines chemicals: a resource type and a key, published from
its root crate ([Dependencies between trait mods](001-dependencies-between-trait-mods.md#dependencies-between-trait-mods)) for other mods to use.

```rust
// The chemistry mod's root crate
#[derive(Clone, PartialEq)]
pub struct Chemical(pub String);
pub const CHEMICALS: CapabilityKey<dyn ResourceHandler<Chemical>> = CapabilityKey::new("chem_api:chemical");
```

A mod that names it under `[dependencies]` builds against the same `Chemical` and `CHEMICALS`, so its providers,
`r.block_entity_capability(&chem_api::CHEMICALS, &vat, …)` (where `vat` is a declared type key or
`r.block_entity_type_key("chem_pipe:vat")`), answer the same capability, and
`level.capability(host, &chem_api::CHEMICALS, pos, side)` finds either mod's handler. `move_resources` moves a
chemical as it moves items, in the caller's transaction. The game has no answer of its own for a mod's key, so only
the mods' providers do, and a lookup with none registered costs one table check. A handler that keeps its amount in
its block entity's saved data (`BlockEntityRef::merge_data`) is covered by the game's journal, so an aborted
transaction puts it back. `ItemCapabilityKey::new` and `EntityCapabilityKey::new` define item and entity
capabilities the same way.

A key is known by its name, which must be namespaced (`chem_api:chemical`), and a name has one handler type. Two
mods that each declare their own `Chemical` under one name, rather than sharing the defining mod's, would never see
each other's handlers. So the game refuses that at startup, naming both mods and both types, and it refuses a key whose name has no
namespace. Both checks run once, when the mods register.

Storage for a holder without a block entity (an entity, a player, a chunk, the level) comes stock, kept in an
attachment the mod declares, as NeoForge mods keep a `ItemStacksResourceHandler` or `SimpleEnergyHandler` in one:
`AttachedItems::new(&SLOTS, holder, size, capacity, valid)` and `AttachedFluids` (`AttachedStacks<R>`, a
`ResourceHandler<R>` over an `AttachmentType<StackList<R>>`, `single` for one index) and
`AttachedEnergy::new(&ENERGY, holder, capacity, max_insert, max_extract)` over an `AttachmentType<u32>`. Give the
item or fluid attachment the codec `ITEM_STACKS` or `FLUID_STACKS` to save it. Every write is an attachment write, so
the transaction journal, which keeps each holder's attachment map, undoes it on an abort; sync follows the attachment
type's `sync` flag. A mod's own resource type implements `StackedResource` to be held.
