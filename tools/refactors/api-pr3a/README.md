# API PR 3a: item boundary

Part of #1608. This directory holds the repeatable proof and edit log for the
item type swap. The host manifest tool is reused from API PR 1, not duplicated.
`manifest_gate.py` compares method names, owners, documentation and hidden flags
while allowing signatures to change. Every deliberate service change needs an
exact old/new line exception; missing or stale exceptions fail the gate.

## Boundary decisions

`ItemKey` remains a process-local index in the existing runtime key table.
Registration can return a forward reference before the game loads its registry;
a copied key must not carry unresolved item metadata.

The API-owned stack guarantees a nonzero count and stores numeric damage as
`Option<u32>`: absence inherits the prototype, `Some(0)` is explicit zero, and
removal is an explicit component-patch entry.
Its constructors and mutators do not promise registry-dependent validity. The
game owns the item registry and checks the effective max count and max damage
(including per-stack component overrides) at the write boundary, returning a
typed error rather than clamping. Mods can query metadata before attempting a
write. This gives native and wasm mods the same implementation and validation
contract without target-specific globals.

Component data remains an optional, owned NBT patch. Plain stacks need no heap
allocation. Component access is through typed descriptors rather than the
engine's component struct. The wasm record is generated from a declared data
form, not a hand-written stack codec.

## Known consumers

Contributor #1730 adds `mod_api::testing::FakeHost` in `src/testing.rs` and
`src/testing/{level,players,services,registries,journal}.rs`, plus migrations of
handwritten hosts in `transaction.rs`, `lib.rs`, `hooks_tests.rs`, wasm
`groups`/`sample`/`points`, `horizon-mod-points/tests/wire.rs`,
`horizon-client/tests/mod_api_hooks.rs`, `brewing.rs` and `menu.rs`. Whichever PR
lands second converts the other; this directory does not implement that PR.

Those hosts must use owned stack constructors and getters, `ItemKey` resolution
for vanilla and mod items, `Option<ItemStack>` for absence, and the keyed `Tags`
service. Implement `max_stack_size(ItemKey)` and `max_damage(ItemKey)` metadata
reads without substituting a made-up maximum for unavailable metadata. Raw
component setters are fallible, and resident writes preserve all original values
on typed refusal. Numeric damage distinguishes inheritance, explicit zero and
removal, including in fake registries and journal assertions.

## Proof commands

The focused gate tests are independent of Cargo:

```sh
python3 -m unittest discover -s tools/refactors/api-pr3a -p 'test_*.py'
```

The final gate consumes manifests generated at the base and the new head.
Signature changes, WIT record changes and hook dispatch equivalence require
separate review/proofs; the manifest comparison does not claim those are equal.
