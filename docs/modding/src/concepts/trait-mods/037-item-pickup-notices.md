<!-- chapter: Item pickup notices -->

Two server hooks tell a mod that a player picked items up off the ground. Each pickup is told to one
of them at most, never both:

- `item_picked_up_post` (NeoForge's `ItemEntityPickupEvent.Post`): the whole stack went into the
  inventory.
- `item_picked_up_partial` (Horizon only): the inventory took part of the stack and the rest stayed
  on the ground, as when 8 apples lie there and only 5 fit.

Neither is told when `item_picked_up` refused the pickup or the inventory took nothing. A creative
player always takes the whole stack, so only `item_picked_up_post` hears them.

Both carry the player's UUID, `original`, a copy of the stack as it lay, and `remaining`, the item
entity's live remainder: `None` after a whole pickup, the part that did not fit after a partial one.
The amount taken is `original`'s count less `remaining`'s. Set `remaining` to change what stays on
the ground, or `None` to empty the entity. Removing the entity in the listener keeps it removed, for
the host's player and LAN guests alike: there is no take animation, the stat and criterion still
run.

A mod that counts every item a player receives, such as a quest counter or a custom pickup sound,
listens to both. The [hook notes](../../reference/hook-notes.md#partial-item-pickups) have the full
contract.
