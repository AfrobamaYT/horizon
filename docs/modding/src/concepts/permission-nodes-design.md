# Permission nodes and handlers: proposed design

**Status: proposal for [#1034](https://github.com/arismoko/horizon/issues/1034), not approved or implemented.** The two gather hooks and the query API described here do not exist yet. This document proposes their behavior for review; adding hooks waits for agreement. It does not change sound hooks or existing permissions.

## Boundary and existing contracts

A permission node is a named, Boolean question a server-side mod explicitly asks about an online player. A default supplies the answer when no external policy is selected. Exactly one selected handler answers every registered node for that server; handlers are not a chain of voters. This is a platform service, not an engine dependency. An unmodified game and its vanilla checks must behave as before. The [layer split](../../../layers.md) and [mod API design](design.md#registering-behaviour) put collection and ownership in the game and prohibit an engine dependency on the mod platform.

Today commands use operator levels, not nodes: `Command::level` is 0–4, with 0 available to everyone (`crates/horizon-mod-vocab/src/command/types.rs`); the command tree and dispatch enforce their existing levels. Existing game-master block checks also use levels (`crates/horizon-client/src/game/administration/command_blocks.rs`). No vanilla command, command tree, command-block restriction or other existing check is implicitly converted to a node. A mod that wants a node check must call it at its own server-side authorization point; a node grant does not grant an operator level, and a node denial does not remove one. Where a mod deliberately requires both a level and a node, both checks must pass.

The existing `permissions_changed` hook is different: the catalog declaration is `crates/horizon-mod-vocab/src/hooks/permissions_changed.rs`, with dispatch target `crates/horizon-client/src/game/native_hooks/permissions_changed.rs`. On dedicated servers it can cancel a pending `PlayerList.op`/`deop` operator-list change, including for an offline player; `operators/clear` is not such a call. It neither reports nor vetoes individual node decisions, and node decisions do not fire it. A canceled level change leaves the old level in force; a committed one affects the next check using that player's current level. A handler must not assume this hook reports all policy or operator-list changes. No node decision cache is specified, so there is no cache to invalidate through this hook.

## Nodes and queries

Each node has a globally unique, case-sensitive name `namespace:path`, where the namespace is the declaring mod's manifest id (`[a-z0-9_]+`) and the nonempty path consists of lowercase ASCII letters, digits, `_`, `-`, `.` and `/`. Names compare byte-for-byte; no case folding, wildcard matching, hierarchy or inheritance is implied. The host records the declaring mod from dispatch rather than trusting a supplied owner. Invalid names or a namespace other than the declaring mod's are registration errors. A consumer may query any registered name; ownership controls declaration, not use.

A node's default is one of `Allow`, `Deny`, or `OperatorLevel(n)` for integer `n` in 0–4. The last returns whether the player's **current server operator level** is at least `n`. A declaration must give a default explicitly; there is no implicit allow or operator bypass. The default is evaluated for the actual player at query time, not when nodes are gathered. An unknown name is an error, never an implicit new node or a grant.

The proposed query takes a registered node name and a live server player belonging to that server, and returns `Allowed`, `Denied`, or an error. A caller must treat an error as denial of the protected action and may report it; error is not a successful `Denied` answer. No offline-player, console, command-block, client-side or arbitrary-context query is defined by this proposal. The query is server-authoritative on dedicated and integrated servers, including LAN players. Clients cannot decide, register, select or override server permissions; a UI may show server-provided information but it is not authorization. This proposal adds no synchronization packet or client permission API.

## Collection and selection

Both hooks are server-side collection hooks, dispatched once per logical server/world startup, after that world's server configuration is available and before players can join, command trees are published or mods can perform permission queries. Integrated-server world openings and dedicated-server starts each build their own table; closing one discards its table and handler. Reopening collects afresh. Resource/data reloads and player joins do not recollect or change the selected handler. Changes to selection configuration require a server restart or world reopen. Server setup must not expose a partial table or accept players if collection fails.

1. `permission_gather_nodes` receives a mutable collector, initially empty, and the server's setup context; each subscribed mod appends node declarations with name and default. It returns no decision and cannot remove or modify another declaration. Dispatch follows the existing mod dependency order, with bundle-order ties ([dependency model](design.md#one-dependency-model-native-or-wasm)). After all listeners return, validate the entire set before publishing it. Duplicate full names are an error even if defaults and owners match, including duplicates in one mod. Report the name and both declaration owners, abort server startup, and publish nothing; never select first or last. Invalid names, invalid levels and callback failure likewise fail startup with the responsible mod and cause.
2. `permission_gather_handler` runs only after the node set validates. It receives a read-only view of all validated names and defaults and a mutable collector of handler candidates, initially empty; it returns no decision. Each candidate has a globally unique `namespace:path` identifier obeying the same naming/ownership rules and one server-lifetime handler resource. Duplicate candidate identifiers, invalid identifiers or collection failure abort startup with diagnostics and no published table. A candidate may answer nodes from any mod, not only its own.
3. Select after all candidates have been gathered. The server configuration contains either no handler id or one exact id. No id selects the built-in default handler, regardless of candidates; an id selects exactly the matching candidate. An unknown configured id is a startup error naming the id and available candidates, not a silent fallback. Collection order never selects a winner. The built-in default is not a candidate id and cannot be shadowed. Once selection succeeds, publish the validated nodes and selected handler together, before the first query.

Registration is a setup operation, not a runtime mutation API. The collection context is subject to the [load-phase access rules](design.md#load-access-belongs-to-the-owning-thread), not a live world transaction. Mod-facing shapes must be usable by native and wasm mods through the same generated API; the proposal does not prescribe Rust ABI or promise the sketch below compiles. Callback ownership and resource teardown must follow the server lifetime, with no client/server or world-to-world sharing of handler state.

## Handler contract and failures

For each query the selected handler receives the validated node identity and its declared default, the live server player identity/handle, and a read-only server context sufficient to read that player's current operator level. It returns `Allow`, `Deny`, `UseDefault`, or an error. `UseDefault` evaluates that node's default for that player at that moment; the built-in handler always does this. A custom handler can grant or deny independently of the default and of the player's operator level. It is called once per query, never once per candidate, and may keep server-lifetime policy state; the host does not cache answers. The node and player references are valid only during the call. Handlers must not recursively query the permission service; recursion is an error rather than an unbounded call. This contract does not provide world writes or a transaction during a decision.

A handler error, trap, panic caught at the mod boundary, invalid return, stale/wrong-server player or recursive query yields a query error and fails the protected action closed. A recursive attempt fails both the inner and the enclosing query even if the handler ignores the inner error. The host identifies the node and selected handler in its diagnostic; it must not switch to the default or another candidate, because that could grant access. An unknown node also yields an error before calling the handler. An uncaught process-level failure cannot be converted to a decision; it must not be represented as a grant. Handler collection/initialization failures are startup failures, not runtime defaults. The future implementation must preserve the existing native/wasm failure boundaries while meeting these observable results.

## Example mod (contract sketch)

This is **pseudocode**, not a compilable or available API. The mod's manifest id is `garden`; `A` and `B` stand for distinct player UUIDs. The signatures and types illustrate the proposed contract rather than an existing Rust interface:

```text
const NODE = "garden:harvest_rare";

impl Hooks for GardenMod {
    fn permission_gather_nodes(&self, setup, nodes) {
        nodes.add(NODE, Default::OperatorLevel(2));
    }

    fn permission_gather_handler(&self, setup, registered_nodes, handlers) {
        handlers.add("garden:policy", GardenPolicy);
    }
}

impl PermissionHandler for GardenPolicy {
    fn decide(&self, context, player, node) -> Result<Decision, Error> {
        if node.name == NODE && player.uuid == A {
            Ok(Decision::Allow)
        } else if node.name == NODE && player.uuid == B {
            Ok(Decision::Deny)
        } else {
            Ok(Decision::UseDefault)
        }
    }
}

fn harvest_rare(server, player, crop) -> Result<(), HarvestError> {
    match server.permissions().query(player, NODE) {
        Ok(Allowed) => harvest(crop),
        Ok(Denied) | Err(_) => Err(HarvestError::PermissionDenied),
    }
}
```

The administrator configures `garden:policy` as the selected handler. A is then allowed even at level 0, B is denied even at level 4, and a third player at level 2 is allowed by the node's default. Without a selected id, the built-in handler uses that default for all three. An error refuses the harvest rather than falling back to a potentially permissive default. This policy does not alter `/op` or any vanilla command.

## Acceptance matrix for an implementation

Each row starts a fresh server unless it explicitly describes a later query. `A` and `B` are distinct online players; the default-handler rows have no configured handler id. Record callback counts and startup results as well as answers. Run equivalent server-side cases for native and wasm, and the indicated lifecycle cases on dedicated and integrated servers.

| Setup / action | Expected observable result |
| --- | --- |
| Register `x:open` = `Allow`, `x:closed` = `Deny`, `x:staff` = `OperatorLevel(2)`; query as A at level 0 | Allowed, Denied, Denied respectively; built-in selected. |
| Same nodes; query `x:staff` as A at levels 1, 2 and 4 | Denied, Allowed, Allowed, respectively; default is evaluated per query. |
| Declare `x:staff` twice from x, with either equal or different defaults | Startup fails, diagnostic names `x:staff` and both declarations; no table published. |
| Another mod y declares `x:staff` | Startup fails for namespace ownership; no table published. Names `x:staff` and `y:staff` coexist. |
| Declare `x:Bad`, `x:`, or `x:staff` with `OperatorLevel(5)` | Startup fails with invalid declaration; no partially usable table. |
| Offer `x:first` then `y:second`; no configured id | Built-in selected, both candidates collected, neither candidate called on a query. |
| Offer those candidates in either dispatch order; configure `y:second` | Only `y:second` called once per valid query; its answer wins independent of order. |
| Configure nonexistent `z:missing` | Startup fails naming the id and available candidates; neither default nor another candidate selected. |
| Offer `x:first` twice | Startup fails naming the duplicate id and owners, even if the handler resources are identical. |
| Select the example `garden:policy`; query A level 0, B level 4, C level 1, C level 2 | Allowed, Denied, Denied, Allowed; handler called once for each query. |
| Query an undeclared name or use a stale/wrong-server player | Error, protected action refused, no handler invocation. |
| Selected handler returns an error, traps or attempts recursive query | Query errors and action is refused; no other handler or default is used as a fallback. |
| On a dedicated server, cancel A's proposed level 0→2 with `permissions_changed`, then query an `OperatorLevel(2)` default; repeat without cancellation | First Denied with level unchanged; second Allowed after the committed change. The node query itself fires no `permissions_changed`. |
| Open a server, join two players, reload data, then close and reopen it | Each gather hook fires once for the first server, not for joins or reload; reopening gathers again and constructs a fresh selected handler. A client alone fires neither hook. |
| Make either collection callback fail | Startup fails identifying the callback/mod and publishes neither a usable node table nor handler; players cannot join. |

These are tests to implement after approval, not claims that the proposed API already passes them. Existing command/operator behavior must also be regression-tested with no node consumers and with a selected handler.
