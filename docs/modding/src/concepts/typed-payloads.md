# Design: typed payloads

Provisional contract for [#813](https://github.com/arismoko/horizon/issues/813), not an implemented API. Ari's side posts the decision on the issue. This note files no issues or PRs.

## Decision: shared codec, registration and delivery

Register a typed value codec, phase, direction, schema version, required/optional peer policy and exactly one owning handler per receiving side at load. Freeze declarations after dependency-ordered registration. Resolve namespaced identifiers once into connection-local numeric slots; dispatch by slot directly to the owner, never broadcast to all mods. Duplicate declarations fail load, including identical duplicates. Only the declaring namespace may register its payload; dependencies use its exported typed token.

Propose ToServer<T> and ToClient<T> tokens with separate send services and ServerHost/ClientHost handlers. Bidirectional declarations produce both tokens and two explicitly sided handlers. Configuration and play are distinct capabilities; common declarations permit both. Configuration handlers get only capabilities valid before world admission. Every callback gets the appropriate Host; codecs receive bounded immutable input and a scoped registry lookup, never a mutable Host or world access.

Share the typed schema/value foundation proposed by [PR #2332 (#811)](https://github.com/arismoko/horizon/pull/2332): the mod owns schema and pure codec, the game owns orchestration, validation and publication. Separate data-format decoding from network encoding while reusing primitive bounds, structured errors, registry references and generated native/wasm adapters. Wire is an ABI representation, not permission to serialize Rust memory, pointers or Any. Erase typed callbacks internally once at registration with a checked type token. Do not invent a second codec framework for networking. Names and signatures remain proposals.

## Evidence and limits

Vanilla 26.3 inspected from `/workspace/horizon` with `python3 tools/rea_vanilla.py --overload-index 1 method net.minecraft.network.protocol.common.custom.CustomPacketPayload codec`: constructs an immutable identifier-to-codec map, reads the identifier before decoding, and delegates absent identifiers to a supplied fallback provider. Encoding writes the identifier then uses the selected codec. The ServerboundCustomPayloadPacket constructor simply stores the payload. This verifies codec selection, not mod negotiation, handler thread policy or fallback safety. Horizon must define those explicitly; this proposal does not inherit a raw fallback.

The public [payload guide](https://docs.neoforged.net/docs/networking/payload/) describes load-time codec/direction/phase registration, main-thread handlers by default, optional network-thread execution and packet distribution. It is labelled **26.1**. The [26.3 PayloadRegistrar Javadoc](https://aldak.netlify.app/javadoc/26.3.x/net/neoforged/neoforge/network/registration/PayloadRegistrar.html) was inaccessible through the web reader. Exact NeoForge 26.3 negotiation, absent-mod behavior and signatures remain **unverified**. No NeoForge source, patches or other project's source was read.

## Negotiation and absent mods

Exchange bounded manifests during configuration, before accepting application payloads. Match identifier, schema version, direction and phase; assign deterministic slots to their intersection. Required missing or incompatible declarations refuse admission with a diagnostic naming both versions. Optional absence yields UnsupportedPeer on send and no packet; never silently substitute raw delivery. Vanilla peers may join only when every declaration needed for admission is optional. Mod package dependency versions and network schema versions are separate checks. Reconfiguration invalidates slots and queued messages from the old connection epoch; renegotiate before play resumes. No version gates retaining an obsolete API or schema conversion shim.

Provisional limits: 4096 manifest entries, identifier 129 UTF-8 bytes, version 64 bytes, manifest total 1 MiB. Reject oversized manifests before creating entries. A forged unknown slot in the negotiated transport is a protocol error; unknown unrelated vanilla/plugin channels retain their existing behavior. Reserve the typed envelope channel against generic send_packet injection without banning unrelated plugin channels.

## Bounds, trust and delivery

Propose application body limits of 30,000 bytes serverbound and 65,536 bytes clientbound, counting nested encoded values; check the complete envelope against protocol limits too. Depth 64, strings 4096 UTF-8 bytes, lists 4096 entries, decoded allocation budget 256 KiB per message. Validate signed lengths, integer overflow, remaining bytes, collection counts and cumulative budget **before** reserve/copy/allocation. A small encoded count must not permit huge decoded objects. Require complete consumption, valid discriminants, finite domain floats where declared and valid registry references. Bound outer framing/decompression before custom decoding; a later codec check cannot repair an earlier unbounded allocation.

Both native input and wasm input are untrusted. Generated bounded primitives enforce these limits; custom codecs require runtime allocation/fuel accounting and must not escape their input scope. Native arbitrary allocation cannot be sandboxed: native code must obey the codec contract, and process-aborting panics remain unrecoverable. Catch unwind/traps at the codec/handler boundary, abort the active transaction, discard pending sends, disable/report the offending callback under runtime policy and close the affected session. Never unwind through the ABI. Invalid remote data disconnects its sender without invoking the handler; local encode failure returns a structured error and sends nothing. Revalidate authority, holder incarnation and handles at apply, not only decode.

Run mod codecs and handlers on the logical receiving main thread, with bounded byte queues from network I/O. Preserve per-connection receive order across payload types and ordinary gameplay actions at the same ingress queue; do not promise ordering across connections. Handler sends become visible after successful root transaction commit; nested commit merges and abort emits zero packets. Queue acceptance is not delivery acknowledgement. Replies run at the next receiving dispatch turn, including integrated play, preventing recursive callback delivery. Configuration completion waits for required sync acknowledgement. Propose 32 messages and 256 KiB queued bytes per peer per tick, plus a 1 MiB connection queue cap; reject local sends with Backpressure, disconnect flooding remote senders. No silent truncation or unbounded retry.

## Routes to replace

Verified owners at this note's base; paths are repository-relative. These are current code owners, not claims that typed delivery exists.

| Route | Owner and treatment |
| --- | --- |
| Server send to player | `crates/horizon-client/src/game/modding/native_net.rs:119`, queue flush `:78`, packet construction `:89`; replace byte queue with typed envelopes and negotiated recipients |
| Client send to server | Same file `:142`, packet send `:160`; replace brand boolean with negotiated token/phase checks |
| Server receive, LAN and dedicated | `crates/horizon-client/src/game/network/lan_actions/packets.rs:770` to `modding/native_net.rs:47`, queued dispatch `:72`; both must enter one ordered typed ingress |
| Client receive | `crates/horizon-client/src/game/network/remote/mod.rs:3842` to `modding/native_net.rs:105`; replace immediate raw callback with ordered delivery |
| Raw hook adapters | `crates/horizon-client/src/game/native_hooks/base.rs:3767` and `:3777`; delete along with `crates/horizon-mod-api/src/hooks/base.rs:677,687` |
| Public sends | `crates/horizon-mod-api/src/host/network.rs:13,35`; delete raw methods and regenerate native/wasm API, docs and sample callers together |
| Configuration advertisement | `crates/horizon-client/src/lan/conn.rs:1131`; replace channel announcement; `game/network/remote/mod.rs:2368,2390` owns capability activation/reset |
| Server dispatch and flush | `crates/horizon-client/src/game/network/lan/mod.rs:3350,3407` runs queued delivery and flush; `game/world/integrated.rs:907` flushes integrated output. Preserve these commit boundaries |
| Integrated and split-screen client | `crates/horizon-client/src/game/network/remote/mod.rs:976` owns connect_local; `game/runtime/split/players.rs:331` calls it. Use the same negotiated transport and queued handlers for each logical client, never direct calls |
| Attachment sends and receive | `crates/horizon-client/src/game/modding/attachment_sync.rs:14,37` selects recipients/builds snapshots; remote receive `game/network/remote/mod.rs:3846`; flush `modding/attachment_sync.rs:84`, client decode `:150`, packet send `crates/horizon-client/src/lan/guest.rs:780`. Migrate to reserved typed declarations, retain tracker authorization |

Generic packet sending in `modding/native_net.rs:217` must also reject forged typed transport packets. This does not remove vanilla packet interop. Implementation must audit configuration I/O and dedicated/integrated ingress scheduling end to end; exact scheduler and socket owners are not yet verified here, and this table must not be treated as a completed exhaustive execution trace.

## Shared synchronization and clean break

Carry server config snapshots during configuration with schema and revision, validate atomically, acknowledge before admission, and discard on disconnect; never overwrite the client's local config file. Live changes publish a complete committed revision or trigger reconfiguration when required. No separate config transport.

Follow [PR #2328 (#796)](https://github.com/arismoko/horizon/pull/2328): attachment envelope includes world/dimension, holder incarnation, residency join, schema and committed revision. Full snapshot on tracking entry precedes deltas; send only to authorized trackers/owner. Stale joins/incarnations/revisions cannot mutate replicas. Gaps request a fresh bounded snapshot; visibility loss clears the replica. The proposed 64 KiB attachment value fits the clientbound body only with explicit envelope overhead accounting: use bounded chunked snapshots with atomic publication, at most 256 KiB assembled and timeout after 100 ticks; larger snapshots fail explicitly. Root rollback sends nothing. Client replicas never authorize server state changes.

Delete raw mod_net messages, net_send_to_* methods, net_from_* hooks, old transport advertising and separate attachment transport in the implementation change. Update catalog, generator, native and wasm consumers, tests, book and horizon-machine-sample together. No compatibility alias, fallback handler, migration shim or dual transport. Native and wasm have identical registration, dependencies, limits and semantics; wasm uses validated opaque tokens and copied bounded values, never guest pointers.

With no declarations, perform one empty-table check at connection setup, no mod manifest traffic, no codec calls, payload allocation or per-tick all-mod scans. Registered delivery costs one slot lookup plus work proportional to the payload and actual recipients. Attachment dirtiness must not trigger encoding when no negotiated recipient needs the value. No-listener means no callback fanout, not skipping validation of an already received transport envelope.

## Acceptance matrix

Required future tests, not runtime results of this note.

| Fixture | Concrete expected result |
| --- | --- |
| ToServer counter=7; reply ToClient counter=8 | Exactly one server handler sees 7 and one client handler sees 8, on respective main threads, native and wasm |
| Send ToClient through serverbound service | Compile-time refusal; forged direction invokes 0 handlers and disconnects sender |
| Two owners declare cells:update | Load fails with both owners; 0 installed duplicate slots |
| Required schema 2 versus peer schema 1 | Admission refused; optional mismatch returns UnsupportedPeer, 0 packets |
| Length -1, count 4097, depth 65, trailing byte | Reject before collection allocation; 0 callbacks; next valid independent session accepts counter=7 |
| Serverbound body 30,000 / 30,001 bytes | First accepted if envelope fits; second PayloadTooLarge, 0 sends |
| 33 sends in one tick | First 32 accepted within byte budgets; last Backpressure |
| Receive A=1, B=2, A=3 on one connection | Handler order 1,2,3 on integrated, LAN and dedicated |
| Handler sends reply then panics/traps | 0 replies published, transaction restored, callback disabled, session closed |
| Config revision 4 sets counter=7 | Client admits only after acknowledgement; local file unchanged; disconnect clears override |
| Attachment join 10 revision 11 value 8; late join 9 value 3 | Replica remains 8; non-trackers receive 0 updates |
| Root abort after attachment edit 7 to 9 | Authoritative value 7, 0 sync packets |
| 100 ticks with no declarations | 0 codec calls, typed packets and payload allocations |
| Generic send_packet for typed channel / unrelated plugin channel | First refused; second retains existing interop behavior |

## Proposed implementation stages

File: none. (1) Ari approves limits, codec ownership and negotiation; verify public 26.3 contract and complete socket/scheduler route trace. (2) Share #811 codec primitives and generated native/wasm registration with bounded adversarial decoding fixtures. (3) Add configuration negotiation and ordered dispatch through integrated, LAN and dedicated routes. (4) Move config and #796 attachment sync to this transport with atomic snapshot tests. (5) Delete every raw caller/catalog surface and update samples in the same clean-break change; demonstrate the matrix and allocation measurements. No Rust is changed by this note.

## Open questions

- **Exact NeoForge 26.3 parity?** Recommendation: obtain public Javadoc before claiming equivalence; accept explicit Horizon choices provisionally, never infer from source or older docs.
- **Shared codec ABI and custom allocation enforcement?** Recommendation: approve one generated bounded foundation with #811; require a reviewed accounting interface before accepting custom network codecs. Native sandboxing is not promised.
- **Required versus optional sync?** Recommendation: required for gameplay interpretation/config dependencies; optional cosmetic attachment replicas only when all consumers tolerate absence.
- **Budgets and snapshot size?** Recommendation: approve the concrete defaults above after representative pack measurement; keep 256 KiB atomic snapshot assembly cap and explicit refusal beyond it. Never silently truncate #796 values.
- **Network-thread handlers?** Recommendation: expose no mod network-thread handler initially; preserve main-thread Host transactions on both targets. Public counterpart differences need explicit review.
- **Remaining route evidence?** Recommendation: trace configuration acknowledgements, framing allocation, dedicated sockets and integrated scheduler before implementation approval; add exact owners and execution tests. The current table identifies migration sites but does not prove exhaustive coverage.
