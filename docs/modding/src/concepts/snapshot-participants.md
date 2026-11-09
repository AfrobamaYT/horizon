# Design: snapshot participants

Provisional proposal for [#999](https://github.com/arismoko/horizon/issues/999), not an implemented guarantee. Named question: **what native-and-wasm participant contract can preserve synchronous rollback given reentry?**

Recommend runtime-owned transactional participant values, with generated typed access, and callback-free rollback. Arbitrary mutable guest fields with snapshot/restore callbacks remain unsupported until the runtime can prove equivalent guarantees on both targets. An exported participant resource alone does not solve reentry. This recommendation narrows the requested arbitrary-state contract and requires maintainer approval before implementation.

## Evidence and counterpart limits

The issue body and all three comments were read, including Ari's release explanation and our claim. The original same-group reentry diagnosis is historical: `crates/horizon-mod-wasm/src/runtime.rs:536` now admits resumed synchronous calls at :557, but :547 refuses guest execution during native panic. Normal reentry is therefore insufficient evidence for unwind restoration. The [#1903 discussion](https://github.com/arismoko/horizon/issues/1903) says the component-model boundary may be replaced by generated shared memory. Nothing here requires WIT, lifting/lowering, a particular store API, guest pointers, or a particular memory layout.

Vanilla has no Horizon participant interface. The pinned 26.3 `ItemStack.copy` bytecode was requested with `python3 tools/rea_vanilla.py method net.minecraft.world.item.ItemStack copy` from `/workspace/horizon`; it returns EMPTY for empty stacks, otherwise copies count and components and preserves pop time. Stack copying cannot establish mod participant callback ordering.

The public [26.3 SnapshotJournal Javadoc](https://aldak.netlify.app/javadoc/26.3.x/net/neoforged/neoforge/transfer/transaction/SnapshotJournal.html) and [transaction documentation URL](https://docs.neoforged.net/docs/resources/server/transactions/) were inaccessible. NeoForge 26.3 snapshot release, child merge, close callback and root notification ordering are **unverified**. The issue requests first-write snapshots per depth, abort including unwind, child merge, and NeoForge release/root notification ordering; these are requirements, not independently verified counterpart facts. Retrieve public SnapshotJournal and TransactionContext documentation before adopting an exact callback contract. No NeoForge source or patches were consulted.

## Routes and existing owners

References are repository-relative at this worktree's HEAD. They identify current integration owners, not completed participant support.

| Route | Verified owner | Proposed treatment |
| --- | --- | --- |
| Native closure transaction, nested success/error and unwind | `crates/horizon-mod-api/src/transaction.rs:44` and :72; Drop at :82 | Preserve synchronous closure semantics and the guard; rollback must finish before a catcher observes state. |
| Game-owned open/close and nested merge | `crates/horizon-client/src/game/runtime/transactions.rs:174`, :179, :192 | Extend this coordinator's Level (:34), never add a mod journal coordinator. |
| Emergency abort | `crates/horizon-client/src/game/runtime/transactions.rs:239` | Restore participant values through the same rollback path. |
| Root effects and rollback | `crates/horizon-client/src/game/runtime/transactions.rs:462`, :513 | Integrate participant lifetime and notification work here; no guest dispatch from rollback. |
| First-write storage examples | `crates/horizon-client/src/game/runtime/transactions.rs:246`, :257, :290, :303, :325, :338, :357, :388 | Existing block entities, attachments, player/menu/entity inventories, equipment, item entities and blocks retain their existing snapshots. Participant limits must not constrain these vanilla stores. |
| Native/wasm handler dispatch while transaction remains active | `crates/horizon-mod-wasm/src/runtime.rs:536`, :547, :557 | Validate the same participant operations on both targets; never infer unwind support from ordinary resumed dispatch. |
| Existing holder identity | `crates/horizon-mod-runtime/src/attachments.rs:66`; block-entity incarnation capture at `crates/horizon-client/src/game/runtime/transactions.rs:66` | Bind holder-backed participants to existing lifecycle identity; no second generation counter. |

These routes cover transaction closure entry/exit, nested merge, emergency abort and deferred effects. Individual transfer callers continue using those shared routes. A code stage must audit generated imports and all unwind/trap containment callers against the then-current runtime before claiming complete native/wasm coverage.

## Proposed contract

Register server-side participant declarations during the existing registration phase. Typed state and snapshot records use API-owned `Wire` representations with generated native/wasm parity. Refuse registration with the proposed named error `UnsupportedParticipantCapability`, identifying the declaration, type and missing capability (bounded owned value, infallible runtime restore, or boundary representation). A persistence codec does not establish Wire support. Do not expose a native-only escape hatch.

A participant key combines its declaration and existing lifecycle holder identity; a retained handle does not pin its holder. Validate owner, server side, world and incarnation on each access. World close invalidates handles. Holder retirement inside a lifted action pass waits for root commit; aborted removal restores the existing binding. Session-owned state binds to the existing mod lifecycle owner, with no independent identity generation.

The runtime owns the authoritative value. Reads return owned typed values; checked replacement requires the borrowed Transaction and journals the old value before mutation. No mutable reference or guest pointer escapes. A mod may cache a read but cannot treat that cache or an arbitrary static field as transactional state. Same-participant recursive edit refuses with `ParticipantBusy`; no mutable borrow crosses dispatch. Snapshot/restore/release perform runtime-owned moves or swaps, without mod codecs, destructors or callbacks during unwind. Allocation and encoding happen before publication; validation failure leaves the old value intact.

At each depth retain exactly one entry value per touched participant. Child abort restores its entry value. Child commit transfers its snapshot if the parent lacks one; otherwise the parent's earlier snapshot survives and the redundant child snapshot is released. Root abort restores all values and drops pending notices. Root commit finalizes values, holder retirement and journal emission in the existing coordinator, including lifted passes. Saves during any open action savepoint refuse with the proposed `ActionSavepointOpen`; they must never serialize tentative values.

Propose one root notice per changed participant, queued after durable in-memory commit, never during rollback. The notice receives the existing ServerHost and committed values; it cannot veto the completed transaction. A notice mutation starts a fresh action. Exact release versus notice ordering and inter-participant ordering remain gated on public NeoForge documentation. Do not advertise counterpart parity or ship arbitrary ordering under the counterpart's name.

Native unwind and wasm trap abort touched game and participant values without executing guest restore code. Contain snapshot preparation failure before mutation. Notice panic/trap is reported through runtime failure policy after commit; it cannot roll committed state back. Do not retry a partially executed notice. Process-aborting panics are outside recoverable unwind guarantees. If arbitrary guest fields are required, registration must refuse them until a reviewed runtime design supports both normal same-group callbacks and failure-time restoration; swallowing a failed restore or returning a default is unacceptable.

Treat all boundary data as untrusted. Validate declaration/schema, handle ownership, length arithmetic, depth and total retained bytes before mutation. Proposed initial mod-only limits: 64 KiB per encoded value, 64 nesting depths and 4 MiB retained participant snapshots per root; refuse with `ParticipantBudgetExceeded`, never truncate. These are recommendations requiring approval and do not cap existing vanilla inventories/components. Native validates the same bounds. Resolve declarations at load; when unused, add only an O(1) empty participant-map check at transaction close, with no registration scans, per-tick allocation or serialization. Used cost scales with touched values and retained bytes.

## Acceptance matrix

Proposed future tests, on both native and wasm; these are not runtime tests run for this note.

| Fixture | Concrete expected result |
| --- | --- |
| Initial 1; outer writes 2; child writes 3 and commits; outer aborts | Value 1; notices 0. |
| Initial 1; outer writes 2; child writes 3 and aborts | Value 2 inside outer; root abort yields 1. |
| Same sequence, both commit | Value 3; root notices 1; no child notice. |
| Two writes 1→2→4 at one depth | Snapshot count 1; abort restores 1. |
| Parent untouched; child 1→3 commits; parent aborts | Value 1, proving snapshot ownership transfer. |
| Participant 1→2 and inventory count 8→5, then native panic / guest trap | Participant 1, inventory 8, notices 0, guest restore calls 0 before catch resumes. |
| Snapshot encoding fails before proposed 2 | Value 1, snapshot count 0, explicit error. |
| Root notice traps after commit to 3 | Value 3, notice attempts 1, no retry or rollback. |
| Payload 65,537 bytes; depth 65; retained budget >4 MiB | Each refuses before its mutation; prior committed value unchanged. |
| Save during open action; remove holder then abort | Save refuses; original holder binding survives; successor cannot be reached through stale handles. |
| Unsupported arbitrary guest field or recursive edit | Named capability refusal / ParticipantBusy; no partial write. |
| No participants registered | Existing transfer outcomes unchanged; participant allocations 0, callback calls 0. |

## Proposed stages

Filed stages: **none**. First verify public 26.3 ordering and approve the narrowed state ownership contract and limits. Then implement one runtime/coordinator slice proving callback-free unwind/trap rollback on both targets. Finally expose generated typed declarations/access, update the coordinator's no-mod-state header and #41 restriction, catalog, docs and sample together, and run the matrix. No stage may export native-only participants or depend on the current wasm transport implementation.

## Open questions

- **Can Portizon's providers use runtime-owned values?** Recommend this as the first contract; arbitrary guest state remains refused. If translation requires live fields, commission the runtime design before claiming #999 implemented.
- **What exact ordering does NeoForge 26.3 promise?** Recommend obtaining public SnapshotJournal/TransactionContext documentation and encoding its release/root-notice trace as independent fixtures before code. Current proposed ordering is not certified equivalent.
- **Which budgets are acceptable?** Recommend the mod-only limits above with explicit registration/runtime errors and no reduction of vanilla capacity; approve using real translated provider sizes.
