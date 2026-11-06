<!-- chapter: Load phases and access -->
<!-- separator: 1 -->

### Load phases and access

Native and wasm hooks share the installed dependency-ordered mod slice, including registration,
config loading and reload-listener registration. The shared resolver keeps bundle-order ties,
checks present optional dependencies too, and refuses cycles for both build targets. Content-only
packages stay after code in the loaded-mod inventory; listener-specific edges still use `ReloadOrder`.

Each owning thread progresses through `Packages → Registration → Setup → Config → World`.

| Phase | Available access |
| --- | --- |
| Packages | Package discovery; no game Host operations. |
| Registration | Declarations through `RegistrationContext`; no Host. |
| Setup | Client setup reads, published asset/data queries and registry/settings/mod-list reads; no world handles or transactions. |
| Config | Settings reads and the existing mod-scoped `ConfigFiles` service; no world handles, transactions or other game operations. |
| Reloading | Published asset/data queries, registry facts, settings and mod-list reads, listener registration, and existing `DataLoadContext` callbacks; no world handles or transactions. |
| World | The owning game's side-typed `ServerHost` or `ClientHost`. |

Registration runs once: `register`, `component_types`, `blocks`, `items` and `standalone_models`
receive only `RegistrationContext`. Client setup runs once after the initial asset apply. The
window's Client/Common configs load once immediately after client setup and before title,
connection, input, render or ordinary tick callbacks. This includes pack-error recovery: the failed
selection is cleared and saved during initialization, but its options screen opens only after Config
completes. A title-screen callback is already in World: world-handle constructors can return `None`,
rather than being forbidden in Setup. A dedicated
server loads Common once before its first world; Server configs load and unload per world.
A second world does not repeat registration or client setup. Config loading can read settings and
use the existing scoped `ConfigFiles` service to read or save its own config files. World handles,
transactions and other game operations panic with `Config` in the diagnostic.
Config file polling is not a phase transition, and Server config unloading leaves the World phase.

Every data or asset reload enters `Reloading`, including listener registration, prepare, apply and
the reload completion notice. Initial loads can nest in Setup; runtime reloads nest in World.
Returning, including after a failed prepare, restores the entering phase. Published asset/data
queries are classified in the catalog (`resource_read`), not granted by helper-name exceptions.
They read the currently published generation, not pending prepare output: completion callbacks can
refresh sprite and recipe caches after apply, including on first world open, without gaining world
handles or transactions. Their rustdoc names their load-phase availability. Server listener
registration retains `&dyn Host`, restricted to these published facts, registry facts, settings and
mod-list queries; callbacks already taking `DataLoadContext` keep that context. Neither permits
opening a world handle or transaction. The first client registration and prepare run on the loader
thread; later registration runs on the render thread, prepare on its worker, and apply on the owning
game thread. Startup's empty client Host is not an extra permission.

The game alone advances phases. An illegal or wrong-thread advance is refused before dispatch;
a forbidden mod call panics naming the phase, rather than returning a Result a listener could ignore.
There is no separate common-setup, load-complete or intermod event: registration and the first server
reload cover common setup, client setup is `client_setup`, dedicated setup is `server_about_to_start`,
and dependencies express intermod ordering. Load complete is the edge after initial Config and before
`server_about_to_start`, not a hook.
