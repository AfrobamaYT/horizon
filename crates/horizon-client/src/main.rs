//! Horizon: native Rust client for Minecraft Java Edition 26.3 gameplay.
#![forbid(unsafe_code)]

/// Identifies this executable as the game for launcher imports. `main` reads it so the
/// linker keeps it; it carries neither an API version nor a per-build id.
static GAME_IDENTITY: [u8; horizon_mod_package::game_identity::MARK.len()] =
    horizon_mod_package::game_identity::MARK;

static HOLDER_CAPABILITY: [u8; 26] = horizon_mod_package::holder::CAPABILITY;
fn main() {
    horizon_mod_package::holder::enter();
    std::hint::black_box((&GAME_IDENTITY, &HOLDER_CAPABILITY));
    horizon_client::entry::main();
}
