<!-- chapter: A block's energy -->
<!-- separator: 1 -->

#### A block's energy

Energy is the capability `ENERGY` (NeoForge's `Capabilities.Energy.BLOCK`), answered by an `EnergyHandler`: an
amount and a capacity, with no resource, and `insert` and `extract` written in a transaction as a
`ResourceHandler`'s are. `r.block_entity_capability(&ENERGY, &battery, …)` (with a declared or looked-up
type key) registers a provider, asked as `ITEMS`' are, and `level.capability(host, &ENERGY, pos, side)` finds one. The game's own answer is a mod block
entity's `block_entity_energy` buffer, as NeoForge's `SimpleEnergyHandler`: a fill through the sides it comes
through, up to the buffer's room and its limit per insert, and a drain by the sides it leaves by, up to the limit per
extract; a side that reaches neither finds no handler. `move_energy(host, &from, &to, max, tx)` moves between two
(`EnergyHandlerUtil.move`) as one step of `move_resources`: all or nothing, a move the target takes only part of
tried again with that part.

Energy held by an item is `ITEM_ENERGY` (NeoForge's `Capabilities.Energy.ITEM`), found where its stack lives with
`place.capability(host, &ITEM_ENERGY)`, as `ITEM_FLUIDS` is, and moved by `move_energy` as a block's is. A mod
answers it for its item with `r.item_capability(&ITEM_ENERGY, "mymod:cell", …)`, keeping the energy in the item's
own stack and rewriting it by an exchange of one, so from a stack of more than one the charged item splits off.
Vanilla has no energy items, so the game answers it for none.

A battery item needn't write its own handler: `ItemComponentEnergy::new(place, component, capacity, max_insert,
max_extract)` (NeoForge's `ItemAccessEnergyHandler`) keeps the energy in an int component of the item (absent reads
0) and rewrites the item with an exchange of one per change, so from a stack the charged item splits off.
`ItemComponentFluidTank::new(place, component, capacity)` (`ItemAccessFluidHandler`) is one tank kept in a compound
component, `{id: "minecraft:water", amount: 1000, components: {…}}` as `SimpleFluidContent` saves it, removed when
the tank empties. Both read and write through `ItemComponentHandler` (read the component at the place, rewrite it by
exchange), which a mod reuses for a resource of its own. Answer `ITEM_ENERGY` or `ITEM_FLUIDS` with them from
`r.item_capability`.
