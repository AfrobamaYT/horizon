# Mod files

A `.hmod` is one mod: content, native code, or both. `horizon_mods::Package` reads the shared manifest and
validates the package; `horizon_mods::Content` reads its resources, and `horizon_mods::index::native::Bundle`
reads its code. Content is preview, native code experimental ([data-formats.md](../../../api/data-formats.md)).

## The zip

- A zip archive. Entries are stored or deflate-compressed, the only methods the readers support.
- A path is relative and `/`-separated: no `\`, no `:`, no empty, `.` or `..` part, and it doesn't start with `/`.
  Files must be regular files, not symlinks, devices or executables. Executable payloads are refused even under
  resource paths.
- Each path appears once. Directory entries carry no payload.

## Project and archive layout

```text
my_mod.hmod
├── horizon-mod.toml
├── icon.png                # optional
├── assets/my_mod/
├── data/my_mod/
├── Cargo.toml              # code mods only
├── src/lib.rs              # code mods only
└── mod.wasm                # wasm mods only
```

- **Content only:** no `Cargo.toml` or `[native]`. Nothing compiles.
- **Native code:** root `Cargo.toml` and `src/`, with `[native]` in the same manifest. The launcher builds it
  into the game. Its resources load through the same content reader as a content mod's.
- **Wasm code:** `mod.wasm` at the top, declared by `[wasm]`, with the `Cargo.toml` and `src/` it was built from
  (a converted mod records `built_with` instead and carries no source). The game runs it with no rebuild.
- Inconsistent code declarations and files refuse the package. `mod.wasm` anywhere else, undeclared, or
  declared but missing, refuses it; so does any other executable file.

There is no mod-id directory level inside the archive. `horizon-mod.toml` is the only manifest and the only
source of the mod's id and version; Cargo's package name and version are not mod identity. IDs are plain
`[a-z0-9_]{1,64}` names, also used as resource namespaces. The launcher writes `horizon-mixin.toml` in its
staging area when needed. Hand-written tool manifests, `horizon-native.json`, and input `native/` or `rust/`
folders are unsupported.

## `horizon-mod.toml`

The shared [manifest fields](../how-to/content-mods.md#the-manifest) include `id`, `name`, `version`, `api = "0.2"`
and `capabilities = ["registries"]`. The content API is checked for exact equality. Unknown fields
refuse the manifest. Code adds `[native]`:

| Field | Type | Rule |
| --- | --- | --- |
| `game` | string | not empty, the Minecraft version (`"26.3"`) |
| `horizon` | string | the exact Horizon commit, 40 lowercase hex characters |
| `mixins` | string | the exact horizon-mixins commit, 40 lowercase hex characters |
| `injection` | array of tables | optional raw injections, written as `[[native.injection]]` |

An injection has a unique, nonempty `id` and a `kind`; it selects exactly one of `hook` or `target`, and
exactly one of `helper` or `trait_method`. A helper is a selected `.rs` file under `src/`, or `build.rs`.
See [Injections](../injections.md) for the fields, bindings and trait injections, and
[Native mods](../how-to/native-mods.md#named-hook-points) for the named hook contracts.

A wasm mod adds `[wasm]` instead, written by `cargo horizon-mod build --wasm`
([Building to `mod.wasm`](../how-to/wasm-mods.md)):

| Field | Type | Rule |
| --- | --- | --- |
| `sha512` | string | `mod.wasm`'s SHA-512, 128 lowercase hex characters |
| `toolchain` | string | not empty, the Rust toolchain it was built with |
| `linked` | table | optional, dependency id → the SHA-512 of the `mod.wasm` it was linked against |
| `built_with` | string | optional, not empty: the converter that made it, for a package with no source |

At load `mod.wasm` must start with `\0asm`, be at most 64 MiB and match `sha512`; each failure refuses the
package with its reason.

Top-level `[dependencies]` declares other code mods by id: a semver requirement string, or
`{ version = "^1", optional = true }`. These versions refer to `horizon-mod.toml`, not Cargo metadata.
Content-only mods cannot declare nonempty dependencies yet
([Dependencies between trait mods](../concepts/trait-mods.md#dependencies-between-trait-mods)).

## Source paths

A code package contains root `Cargo.toml`, optional `build.rs`, `.rs` and `.toml` files under `src/`,
and root text files named `README*`, `LICENSE*`, `LICENCE*`, `COPYING*` or `NOTICE*`.
The marked root `impl Hooks` uses `#[mod_api::horizon_mod]`; the attribute generates the factory
([Trait mods](../concepts/trait-mods.md#trait-mods-draft)).

Folder packaging selects source and resource files, excluding `.git/` and `target/`. It validates selected
files before copying them. Whole archives are checked, not treated as source folders with ignored extra entries.

## Content paths and icon

`assets/` or `data/`, then at least two parts of `[a-z0-9_.-]` (vanilla's resource-location characters).
Code and content use the same top-level resource trees; there is no separate native data folder.

`icon.png` is optional for both kinds. It has no manifest key. The package reader fully validates it once:
a PNG, nonzero square dimensions, at most 512 × 512 pixels and 256 KiB encoded. The launcher shows it in
the mod list. The scaffold does not create a default icon.

## Limits

| What | Limit |
| --- | --- |
| A registry file, as downloaded | 64 MiB |
| Selected native source files, excluding the manifest, icon and resources | 511 files, 64 MiB unpacked in total |
| Content files | 65,536 files, 16 MiB each, 512 MiB in total |
| `horizon-mod.toml` | 32 MiB |
| `icon.png` | 256 KiB encoded, 512 × 512 pixels |
