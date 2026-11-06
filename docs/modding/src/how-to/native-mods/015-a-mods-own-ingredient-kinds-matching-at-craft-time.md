<!-- chapter: A mod's own ingredient kinds, matching at craft time -->
<!-- separator: 1 -->

### A mod's own ingredient kinds, matching at craft time

The load-time list only names items. Matching a concrete stack — its components, usually —
is `native_ingredient_test(context, kind, data, stack, accept)`: the ingredient's `test(host, stack)` reaches it with the loop `context` of the side `host` runs on, the
kind and JSON the load-time hook saw, the stack being matched and the verdict, already `true`
(the list named the item). Set `accept` to `false` to refuse the stack (`ICustomIngredient.test`).
The hook may be called many times a craft; the answer must be the same for the same stack.
