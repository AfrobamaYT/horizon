//! crate_demo's own crate: the mod's logic, compiled apart from the game and kept when only the
//! mod's helpers change.

#[derive(Default)]
struct Mod;

#[horizon_client::mod_api::horizon_mod]
impl horizon_client::mod_api::Hooks for Mod {}

mod salt;
use salt::SALT;

/// CRC-32 of the salt then `bytes`.
pub fn checksum(bytes: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(SALT);
    hasher.update(bytes);
    hasher.finalize()
}
