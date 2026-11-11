# Registered payloads and the typed-payload design

The registered **raw** transport implements [#2882](https://github.com/arismoko/horizon/issues/2882).
Typed codecs/tokens, configuration snapshots, and attachment migration remain work for
[#813](https://github.com/arismoko/horizon/issues/813). They are not implemented by the raw API.
The transport contract is detailed in [Registered raw custom payload transport](../../../design/raw-custom-payload-transport.md).

## Register raw payloads once

Register during `Hooks::register`, in dependency order. `payload_handlers().registrar("1")`
returns a `PayloadRegistrar` view. Versions are exact, nonblank strings, not semver ranges.
Main-thread handling and required peer support are the defaults.

```rust,ignore
use mod_api::{IPayloadContext, IPayloadHandler, PayloadBytes, PayloadId, Registrar};
use std::sync::Arc;

struct Update;
impl IPayloadHandler for Update {
    fn handle(&self, bytes: PayloadBytes, context: &mut dyn IPayloadContext) -> Result<(), String> {
        // Decode this application's bytes here. Network handlers must enqueue
        // main-thread work before requesting a world Host.
        let _ = (bytes, context);
        Ok(())
    }
}
fn register(registrar: &mut dyn Registrar) {
    registrar.payload_handlers().registrar("1")
        .play_to_server(PayloadId("example:update".into()), Arc::new(Update));
}
```

The registrar has `play_*`, `configuration_*`, and `common_*` methods for
`to_client`, `to_server`, and `bidirectional`. Common occupies both protocols.
A client handler may be supplied at registration or through
`RegisterClientPayloadHandlersEvent` before the installed table freezes.
`optional()`, `versioned(...)`, and `executes_on(HandlerThread::Network)` return
modified views; they do not modify the original view. Duplicate protocol/id
registrations, invalid ownership or versions, and missing receiving handlers are
refused. An id grants no authority merely because a caller constructs a `PayloadId`.

Native and wasm use the same declarations, opaque `PayloadBytes`, handlers, and
results. A callback receives the real connection's `IPayloadContext`, not a
broadcast event. Dispatch selects the one registered owner.

## Admission, sending, and context

Configuration exchanges declarations and freezes the compatible peer intersection.
A missing required declaration refuses admission. Matching protocol/id declarations
must agree on version and direction even when optional; mismatches refuse with the
id, reason and both declarations. Only one-sided optional absence permits admission,
and send then returns `UnsupportedPeer`
without a packet. When neither peer declares payloads, no mod manifest is exchanged.
There is no per-tick scan of loaded mods.

Clients send through `ClientHost::send_payload_to_server`; servers use
`PlayerRef::send_payload`. `IPayloadContext` offers `player`, `reply`, `disconnect`,
`enqueue_work`, and sided Host access. Configuration has no play player. A NETWORK
handler has no direct world access: enqueue work to obtain the receiving main-thread
context. Wrong-thread and wrong-side requests return explicit errors.

`Queued` means accepted locally, not acknowledged by the peer. Sending reserves
physical/provider credit before acceptance. Root commit publishes staged sends;
rollback publishes none. Replies enter a subsequent receiving turn, including
integrated play. Play delivery shares the connection's ordinary gameplay inbox, so
movement/actions before a raw message are applied before its MAIN handler, and
later inputs wait until that handler's root transaction finishes.

Work and prepared sends bind to a physical connection and protocol incarnation.
Reconfiguration invalidates old queued work; returning to Play cannot revive an old
prepared packet. Original input credit remains held through queued work and derived
actor effects, not merely until dequeue. Queue pressure returns `Backpressure` or
refuses the flooding session; there is no 32-message-per-tick rule or silent truncation.

## Capacity and interop

Registered application bytes use their own namespaced vanilla custom-payload channel.
There is **no application envelope**. Unknown unrelated plugin channels retain
vanilla's discard limits: 32,767 serverbound and 1,048,576 clientbound body bytes.
Those numbers do **not** cap a registered body.

A network frame must fit 2,097,151 encoded bytes. Compressed transport also limits
the complete uncompressed packet to 8,388,608 bytes. Both include packet/channel
framing; highly incompressible data may hit the encoded limit first. Private memory
connections do not inherit network framing caps. Resource quotas still apply and
are resource refusals, not a smaller codec body limit. Frame preparation is reused
for writing rather than recompressing solely for validation.

`ClientHost::send_packet` still supports unrelated vanilla/plugin interoperability.
It validates the packet and reserves outgoing credit, but publication waits for root
commit and the same physical protocol incarnation. It refuses every locally owned
registered channel and engine-reserved channel; it cannot bypass negotiation,
direction, or service authority. The former envelope hooks and tick-capped send
methods are removed, without aliases or fallback handlers. Private custom books
advertise their own version on `horizon:recipe_book_settings`; book support does not
depend on a mod transport flag.

## Remaining typed work in #813

The earlier accepted design is amended by the raw transport's actual codec and
framing contracts above: remove its generic registered-body ceilings, typed
application envelope/numeric-slot proposal, and 32-per-tick acceptance row.
Keep typed registration/helpers outside object-safe Host dispatch; delegate to the
same raw transport rather than adding another route. A typed token must bind its
checked declaration/type, phase, direction, and load identity. Forged or stale
wasm tokens must refuse without invoking a handler.

Share schema/value codec primitives with #811. Wire is an ABI representation, not
permission to serialize Rust pointers, memory layouts, or `Any`. Supported values
need explicit Wire forms on both targets, or a named refusal at load. Structured
codecs must check signed lengths, overflow, remaining bytes, cumulative allocation,
depth, discriminants, and registry references before allocating; require complete
consumption. Per-schema bounds must not silently reinstate a lower generic transport
ceiling. Native arbitrary allocation/process-aborting panics cannot be sandboxed.
Errors and wasm traps must abort the active transaction and discard staged sends.

Configuration snapshots still need revisioned atomic publication and acknowledgement
before admission, without overwriting local config files. Attachment migration still
needs #796's world/dimension, holder incarnation, residency join, schema/revision,
tracker authorization, full-before-delta ordering, bounded assembly, and rollback
proofs. Both use this transport; neither gets a separate networking implementation.

Acceptance for those future layers must exercise native and wasm over the real
integrated, LAN, and dedicated routes: exact typed decoding, mismatched/optional
peers, invalid tokens, malformed structured values, sender identity, ordered
application, next-turn replies, root rollback/traps, stale epochs, and resource
pressure with progress after drain. Measure wasm copying/allocation honestly; Wire
transfer is not zero-copy. Raw transport tests do not prove the unfinished typed,
configuration-snapshot, or attachment contracts.
