//! Full-chunk save seam. Event dispatch will be added separately.
use super::Game;

impl Game<'_> {
    /// Dispatch while the selected dirty chunks are still live, before snapshotting.
    pub(super) fn native_full_chunk_saves(&mut self, keys: &[(i32, i32)]) {
        if self.remote.is_some() {
            return;
        }
        for &(x, z) in keys {
            if self.world.modified.contains(&(x, z)) && self.world.chunks.contains_key(&(x, z)) {
                self.native_full_chunk_save([x, z]);
            }
        }
    }

    fn native_full_chunk_save(&mut self, _pos: [i32; 2]) {}
}
