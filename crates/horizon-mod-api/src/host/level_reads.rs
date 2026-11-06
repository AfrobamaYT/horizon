//! The `level_reads` area: host methods the game implements (`game/host/level_reads.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `level_reads` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LevelReadsBackend {
    /// Side: both.
    /// The block state ID at `pos` (`Level.getBlockState`), without formatting a string.
    /// None for a chunk that isn't loaded or a `y` outside the world. On a client, the
    /// state is the one the server sent for a chunk the client holds.
    fn level_block_state(&self, level: LevelRef, pos: [i32; 3]) -> Option<BlockStateId> {
        crate::phase::unprovided(
            "Backend::level_block_state",
            "level_block_state",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Preview a held tool's transformation without changing the block or stack. The existing
    /// block-tool hook fires with `simulated` true; listeners must not mutate the level in a
    /// simulation. The player must be live in the level in place, and the block must be loaded;
    /// parked levels cannot provide the hook's level context. A stale handle, invalid face,
    /// unavailable player or level, or no transformation returns None.
    fn level_block_transform(
        &self,
        level: LevelRef,
        player: PlayerRef,
        hand: Hand,
        stack: &ItemStack,
        pos: [i32; 3],
        hit: BlockHit,
    ) -> Option<BlockTransformPreview> {
        crate::phase::unprovided(
            "Backend::level_block_transform",
            "level_block_transform",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The block at `pos` without its properties: `minecraft:oak_stairs`.
    /// None as for `block_state`. API 381.
    fn level_block(&self, level: LevelRef, pos: [i32; 3]) -> Option<String> {
        crate::phase::unprovided(
            "Backend::level_block",
            "level_block",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Property `key` of the block state at `pos` (`BlockState.getValue`):
    /// `east` for `facing`. None for a block without it, or as for
    /// `block_state`. API 381.
    fn level_block_property(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        key: &str,
    ) -> Option<String> {
        crate::phase::unprovided(
            "Backend::level_block_property",
            "level_block_property",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether this is the level in place: the one the server ticks now, whose block entities
    /// the game's own capabilities read and write and whose capabilities
    /// `LevelRef::capability` looks up. False for a level parked while the server ticks
    /// another (its blocks can still be read), for a handle of an earlier open, and on a
    /// client.
    fn level_in_place(&self, level: LevelRef) -> bool {
        crate::phase::unprovided(
            "Backend::level_in_place",
            "level_in_place",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether `pos` is inside the world's height and its chunk is loaded
    /// (`Level.isLoaded`, which is false outside `isInValidBounds`): a block
    /// read or write there finds the world. False on a client or for a stale
    /// handle: the position is not loaded here. On a client, a chunk it holds is loaded
    /// (`ClientLevel.hasChunkAt`). API 381.
    fn level_is_loaded(&self, level: LevelRef, pos: [i32; 3]) -> bool {
        crate::phase::unprovided(
            "Backend::level_is_loaded",
            "level_is_loaded",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The stored light of one layer at `pos` (`BlockAndLightGetter.getBrightness`), 0 to 15.
    /// None as for `block_state`: a position that isn't loaded has no light to read.
    fn level_brightness(
        &self,
        level: LevelRef,
        layer: LightLayer,
        pos: [i32; 3],
    ) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::level_brightness",
            "level_brightness",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The lowest block `y` and the height of the level
    /// (`LevelHeightAccessor.getMinY`, `getHeight`). Reads any loaded server level
    /// or the client's current level. None for a stale handle, before client login,
    /// or for a dimension unavailable on this side. Does not load chunks. API 381.
    fn level_height(&self, level: LevelRef) -> Option<(i32, i32)> {
        crate::phase::unprovided(
            "Backend::level_height",
            "level_height",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The ticks the server has run (`Level.getGameTime`), shared by every
    /// level as in vanilla. API 381.
    fn level_game_time(&self, level: LevelRef) -> Option<i64> {
        crate::phase::unprovided(
            "Backend::level_game_time",
            "level_game_time",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The time of day in ticks (`Level.getOverworldClockTime`), shared by
    /// every level; it keeps counting past 24000. On a client, the synchronized
    /// Overworld clock, advanced at its server-supplied rate, even when the
    /// displayed dimension has a different default clock. API 381.
    fn level_day_time(&self, level: LevelRef) -> Option<i64> {
        crate::phase::unprovided(
            "Backend::level_day_time",
            "level_day_time",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `FluidState.getFlow` at `pos`: the current that pushes an entity there, after the
    /// `fluid_flow` hook, as entities feel it. Zero where no fluid flows. None as for
    /// `block_state`.
    fn level_fluid_flow(&self, level: LevelRef, pos: [i32; 3]) -> Option<Vec3> {
        crate::phase::unprovided(
            "Backend::level_fluid_flow",
            "level_fluid_flow",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
