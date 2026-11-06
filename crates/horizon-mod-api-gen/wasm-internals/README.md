# Staged wasm internals table

Each mod opts in the game internals it needs in `<mod_id>.txt`, one exact
`internals.json` path per line. Blank lines and `#` comments (including trailing
comments) are ignored. The stage reads the union of all `.txt` files; mods added
independently do not edit a shared list. A missing directory means an empty table.

A field declaration exposes its owner node with that field's arms only. Field
numbers remain the declaration-order numbers of the complete type, across all
cases of an enum. Merely declaring a method exposes its receiver node, not all of
that receiver's fields. Declaring a struct or enum, or reaching a type below a
field or in a function's argument/result, exposes its crossable layout
transitively. Such a full-layout reach also takes precedence over a partial owner.
Only declared functions get call thunks; reaching a type does not opt in its
methods.

For a declared raw injection target, the table also includes its head record and
its supported return place. The receiver is separate from the record. Return
places expose the returned value as field 0; shared receivers stay read-only.
Named type, member and thunk entries retain their manifest path and hash. Raw
places retain the point's readable wire shape; synthetic nodes have an empty hash
and are reached only from checked entries.

The stage reports missing, conditional, opaque and unsupported declarations with
their individual reasons, but does not fail. The mod loader is responsible for
refusing a mod whose declared internals are unavailable. A seed file is not proof
that every native operation of a mod can run in wasm: `treefall.txt` records the
remaining lower-crate method, opaque input and closure boundaries as comments.
