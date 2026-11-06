<!-- chapter: Block-state facts -->

### Block-state facts

A `BlockStateId` is a registry value, not a live object, so the facts about one that need no world are a small
read-only service on any host, both sides: `host.block_states()` (import `HostBlockStates`). Each reads the game's own
tables. Fact queries and matching allocate nothing; `definition` copies the property data for the caller:

| Method | Vanilla |
|---|---|
| `replaceable(state) -> Option<bool>` | `BlockState.canBeReplaced()` |
| `destroy_speed(state) -> Option<f32>` | `BlockBehaviour.Properties.destroyTime` (`getDestroySpeed`): stone 1.5, -1 unbreakable |
| `sound(state) -> Option<BlockSoundType>` | `BlockState.getSoundType()`: volume, pitch and the break, step, place, hit and fall events |
| `has_block_entity(state) -> Option<bool>` | `BlockState.hasBlockEntity()` |
| `states_of(block) -> Option<BlockStateRange>` | `StateDefinition.getPossibleStates`, as the block's consecutive ids |
| `definition(block)` | Property names and legal serialized values |
| `matches(state, block, constraints)` | Partial property match with typed invalid-property and invalid-value errors |

Use `StateConstraint` for each partial constraint; `StateMatchError` identifies an invalid property or value by index. `StateProperty` describes a property's legal values. These types are shared by state-definition services.

None for an unknown state or block, and on a host with no game behind it (a mod's test double).

