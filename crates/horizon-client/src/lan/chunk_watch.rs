//! The chunks each player watches, for the mods' `chunk_watch`, `chunk_sent`
//! and `chunk_unwatch` (NeoForge's `ChunkWatchEvent`). The hub records them
//! only while a mod listens; the game tells the mods after the hub's step.

use mod_api::HookId;

use super::guest::Guest;

/// What happened to a player's chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkWatchKind {
    /// The chunk entered the player's view (`ChunkWatchEvent.Watch`).
    Watch,
    /// Its packet went to the player (`ChunkWatchEvent.Sent`).
    Sent,
    /// The player stopped watching it (`ChunkWatchEvent.UnWatch`).
    Unwatch,
}

/// One chunk event for one player, in the order the hub made them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkWatch {
    pub kind: ChunkWatchKind,
    pub uuid: u128,
    pub entity_id: i32,
    /// The level the chunk is in (`minecraft:overworld`).
    pub dimension: String,
    pub chunk: [i32; 2],
}

/// Whether a mod listens to any of the three.
pub fn listened() -> bool {
    [HookId::ChunkWatch, HookId::ChunkSent, HookId::ChunkUnwatch]
        .into_iter()
        .any(mod_api::__private::listened)
}

/// One event for `g`'s chunk `(x, z)` in its level.
pub(super) fn record(
    out: &mut Vec<ChunkWatch>,
    g: &Guest,
    kind: ChunkWatchKind,
    (x, z): (i32, i32),
) {
    out.push(ChunkWatch {
        kind,
        uuid: g.uuid,
        entity_id: g.entity_id,
        dimension: g.dimension.clone(),
        chunk: [x, z],
    });
}

/// `g` stops watching chunk `(x, z)` (`ChunkMap.dropChunk`), if it did.
pub(super) fn unwatch(out: &mut Vec<ChunkWatch>, g: &mut Guest, key: (i32, i32)) {
    if g.watched.remove(&key) {
        record(out, g, ChunkWatchKind::Unwatch, key);
    }
}

/// `g` stops watching every chunk (`applyChunkTrackingView(EMPTY)`): before
/// it leaves its level, respawns or disconnects. In the order vanilla walks
/// its square view (`ChunkTrackingView.Positioned.forEach`): by x, then z.
pub(super) fn unwatch_all(out: &mut Vec<ChunkWatch>, g: &mut Guest) {
    if g.watched.is_empty() {
        return;
    }
    let mut keys: Vec<_> = g.watched.drain().collect();
    keys.sort_unstable();
    for key in keys {
        record(out, g, ChunkWatchKind::Unwatch, key);
    }
}
