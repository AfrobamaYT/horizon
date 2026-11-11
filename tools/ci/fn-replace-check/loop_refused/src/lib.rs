//! Loop rows the build refuses by name: nothing here is compiled into the game.
use mod_api::loops::block_entity_tick::BlockEntityTickSlot;

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {}

#[derive(Default)]
pub struct Slot;

impl BlockEntityTickSlot for Slot {}
