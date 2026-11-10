<!-- chapter: Machine-readable SDK catalog -->

## Machine-readable SDK catalog

Tools discover a build's hooks and injection points through `mod-catalog.json`,
shipped at the root of each prebuilt SDK and beside `points.json` in a developer
source build's catalog directory. Converters, linters and IDE helpers must read
this file rather than source paths. Internal source layout is not a tool API.

The JSON document has `schema_version: 1`, `api_version`, `hooks` and `points`.
Consumers must refuse unknown schema versions. The API version is the mod
manifest API version; it is independent of the catalog format version.

Hooks are sorted by `id` and describe `side`, nullable `scope` (Part `kind` and
`key`), `contract`, ordered `arguments` with API `name` and `type`, and nullable
`output`. Types are Rust API event field types, including their borrow lifetimes.
Points are sorted by name and target; a target distinguishes equal names.
Each includes its supported `access` values (`api`, `internals`), API `record`
and ordered `args`, return contract and type, raw shape when available, and
nullable `api_refusal`. Refused and native-only points remain in the file.

The build generator owns this catalog. Output has no timestamps or absolute
paths. A format change requires a schema version change; tools should use the
catalog accompanying the build they target. CI regenerates the catalog in-test and compares the two documents for determinism.
The full file ships only in the SDK and developer source build; no committed
fingerprint needs refreshing when independently authored hooks land.
