//! Depends on `counter_lib` and listens at `handle_chat_input`, appending `L`.
use mod_api::{Hooks, HostSide};
use mod_points::HandleChatInputArgs;

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {}

pub fn handle_chat_input(_host: HostSide<'_>, args: &mut HandleChatInputArgs<'_>) {
    let _ = &counter_lib::COUNTER;
    args.message = format!("{}L", args.message).into();
}

#[cfg(target_arch = "wasm32")]
include!(concat!(env!("OUT_DIR"), "/guest_injects.rs"));
