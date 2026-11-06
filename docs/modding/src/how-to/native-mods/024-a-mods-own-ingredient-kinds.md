<!-- chapter: A mod's own ingredient kinds -->
<!-- separator: 1 -->

### A mod's own ingredient kinds

A recipe's ingredient that names a type in a namespace outside `minecraft:` reaches
`native_ingredient(context, kind, data, items)` at load time. Fill `items` with the item ids the
ingredient matches (item ids only: components are not checked; a match-time hook follows
separately). Duplicates are removed keeping first-seen order. The hook gets the `DataLoadContext` of the load that parses the recipes and hears each occurrence of the kind once per load, nested ones and ones under any slot a recipe reads an ingredient from included, before the load's tables parse the recipes: the client's own recipe load has `origin()` `ClientPreload`, the server's `ServerDataLoad`. A server loads its recipes once however many tables and copies read them (its recipe tables, the stations and the copy synchronised to guests, on a dedicated server too), and a `/reload` asks again once; a client that only joins a remote server is never told `ServerDataLoad`. The answer must be the same every time. An unclaimed kind leaves the list empty and the recipe fails, as an unknown kind does.
