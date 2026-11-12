# Sync 27 integration decisions

## CTM sprite deduplication: refused at the protected boundary

The upstream change preloads CTM rules in `assets.rs`, calls
`Sprites::keep_apart` before model textures are inserted, and passes the loaded
rules into `ConnectedTextures::load`. The fork's protected `assets.rs` inserts
model textures before calling that loader. At that point sprite layers have
already been interned; changing only `connected.rs` cannot undo the prior
coalescing without remapping every model's layer indices. Changing `load`'s
signature alone would also break its protected caller.

This sync therefore retains the fork's existing CTM loader and does not claim
upstream's deduplication behavior. A correct integration needs authorization to
change the assets construction order, or a separately designed layer remapping
pass. The installation-scoped interner boundary remains unchanged.

## Contextless fireworks service: refused

Upstream adds an audible flag to its contextless fireworks request helper.
The fork removed that helper: `native_client_particles.rs` exposes the typed
single-particle request, not a fireworks explosion request, and the typed host
has no corresponding fireworks service. Reintroducing the old numeric service
would violate the clean break. A new typed fireworks service requires an
API-owned explosion payload and its Wire contract, not an engine explosion
slice exposed through the API. This sync does not add that new capability.
Existing engine firework rendering already passes the audible flag based on
rocket silence in `game/entities/fireworks.rs`; that path is retained.
