# The mod API (rustdoc)

The reference for every hook, event, handle and `Host` method is the rustdoc of the `horizon-mod-api` crate
(lib name `mod_api`), generated from the `///` docs of the declarations in `hooks/*.rs`:

```sh
cargo doc -p horizon-mod-api --no-deps --open
```

Each generated `Hooks` method, event and `Host` method carries its `doc`, with the side and cancel contract the
catalog records. CI builds it with `-D warnings`, so a broken intra-doc link fails. The per-hook prose that predates
the `doc` field is in [Hook notes](hook-notes.md) until it moves into the fragments.

## Process environment

`RegistrationContext` exposes `dist()`, `game_version()`, `horizon_version()` and
`game_tests_enabled()` before declarations are registered. `Dist::Client` names the physical
client process, including its integrated server; `Dist::DedicatedServer` names a dedicated
server process. This is not the logical side of the current hook.

The `Host` config area exposes the same distribution and versions on both sides, and
`game_tests_enabled()` on the server host. Minecraft's version is exactly `horizon_core::VANILLA_VERSION`;
Horizon's version is the string printed by crash reports. All values are captured once before
mod registration and are transported identically to wasm.

Game tests default to disabled. Pass `--game-tests` (before or after `--dedicated`), or set
`HORIZON_GAME_TESTS=1` or `HORIZON_GAME_TESTS=true` for CI. Other environment values do not enable
the mode. This flag lets mods declare test-only content; `register_game_tests` still runs on
every world open regardless of the flag, and `/test` remains available.

The switch is recognized only in an option position: `--dedicated --world --game-tests`
opens the world named `--game-tests`, without enabling test mode. Dedicated `--universe`,
`--port`, `--serverId` and `--pidFile` likewise retain their operands verbatim.

The resource-backed acceptance tests
`native_process_environment_in_registration_and_game_hosts` and
`native_and_wasm_process_environment_in_registration_and_game_hosts` live under
`game::native_hooks::runtime_environment::tests` in the `horizon-client` library test target.
Set `HORIZON_TEST_RESOURCES` to prepared resources and run them with `--ignored`.
The latter also needs `HORIZON_WASM_FIXTURE` and `HORIZON_WASM_STD` rebuilt from this tree
([fixture build instructions](../how-to/wasm-mods.md)). It compares a real guest's registration
snapshot and later generated Host replies with native values in client, dedicated and integrated
Game harnesses, including disabled mode, both flag positions and CI environment opt-in.

## Plain-data source declarations

The bindings generator reads the Rust type declaration; codecs are not maintained separately for native and
wasm mods. A type with a separate public form declares `#[horizon_mod_api_macros::wasm(via = Form)]` on the
type or its re-export, with `TryFrom<&Type> for Form` and `TryFrom<Form> for Type` beside the type.

An owned tuple struct whose fields need load-owned validation declares
`#[horizon_mod_api_macros::wasm(host_constructor = from_slot)]`. The named associated constructor takes the
fields in declaration order and returns `Result<Self, E>`, where `E` implements `Display`. The macro checks
that signature, and generated incoming host lifts use the constructor and the ordinary typed refusal path.
This applies recursively inside options, lists, records and enum payloads, including hook mutation results.
Lowering is still structural numeric transport. Guest lifts of host-owned values remain structural and do not
need a guest registry table.

`BlockKey` and `EffectKey` use this rule to reject unknown incoming slots against their canonical load-owned
name tables. While `Hooks::register` is in flight, validation reads the active registrar's pending requests;
that same table moves into the installed load state afterward, without a second registry. A valid slot does not
imply that a world's resolved mapping is available: startup, another owning
thread or a refused block reload may have no mapping. Typed operations use trusted numeric indices once that
context exists; `key.id(host) -> String` remains an explicit diagnostic read through the normal generated backend,
not a per-operation name lookup. The effect registry remains fixed for the process.
