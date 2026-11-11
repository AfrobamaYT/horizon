# Structs a mod builds and structs a mod receives

Contract for [#2862](https://github.com/arismoko/horizon/issues/2862). Every public struct in `horizon-mod-api`
and its vocabulary crate is exactly one of two kinds.

1. **A type a mod builds** (definitions, options, screens, declarations): public fields and `Default`. Write a
   literal with `..Default::default()` at the end, so a field added later never breaks the mod:

   ```rust,ignore
   let screen = Screen { width: 176.0, height: 166.0, ..Screen::default() };
   ```

   Where a field has no neutral value (an identity, such as a registry id), the type has a constructor that takes
   the required fields and leaves the rest at their defaults, and no `Default` that would invent one.
2. **A type a mod only receives** (event payloads, snapshots, contexts such as `HurtSource`, `Victim` or `Hit`):
   `#[non_exhaustive]`. A mod reads its fields but can't build it or match it without `..`. The game builds it
   through its mirror in `__private::built` and `From`.

The two can't overlap: `#[non_exhaustive]` forbids struct literals outside the crate.

## The check

`tools/mod_api_struct_kinds.py` reads the sources and lists the structs that are neither: a mod can build them
with a literal (named fields, all `pub`, not `#[non_exhaustive]`), but they have no `Default` and no constructor.
`python tools/mod_api_struct_kinds.py --summary` prints the counts.

`tests/test_mod_api_struct_kinds.py` fails when that number rises above `NEITHER_LIMIT`, so a new public struct
must take one of the kinds. It also fails when the number falls without the limit following it: the batches that
classify the existing structs lower `NEITHER_LIMIT` as they go, and it never goes back up.

The batches go by module. Kind 2 is applied mechanically; which fields of a kind 1 type are required is decided by
hand. Each batch updates the samples and this book in the same PR, with no aliases or shims.
