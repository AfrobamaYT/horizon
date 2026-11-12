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
