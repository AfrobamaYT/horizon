# Atomic internals ownership move

Part of #1607, U5d. This draft starts with the replay/equivalence framework;
the ownership transformation and focused regressions are not complete yet.

The final transformation moves the fingerprinted `internals` array from the
native table to the manifest root, for both code targets. It removes the wasm
copy and the native-only reader. No dual-key reader or compatibility shim is
introduced. Existing native build pins and support modules remain unchanged.

`edits.json` enumerates exact source edits and their occurrence counts against
the pinned base; `move.py --apply` replays them. `move.py` checks the result and
reports the removed/added-line multiset equality modulo those declared edits.
Review the operations, not an inferred file rename. Fixture path/hash/kind
values and CLI writer comments must remain intact. Newly authored regressions
and their mutant results are separately reported and count as hand-written work.

The parser, CLI writers, launcher and wasm readers must move in one commit.
An old `[native].internals` form must fail ordinary strict-schema validation.
The final ready body will record exact-head CI and the focused proof results;
this initial draft makes no completion or test claim for the feature.
