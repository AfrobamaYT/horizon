//! Uses two declared fields and a thunk; the host observes only committed writes.
use mod_internals::view::{Game, HandleChatInputRaw};

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {}

// The name also verifies that runtime imports do not shadow a raw listener.
fn call(raw: HandleChatInputRaw, game: Option<Game>) {
    let game = game.expect("a method receiver");
    game.set_left(game.left().unwrap() + 2).unwrap();
    game.set_right(game.right().unwrap() + 3).unwrap();
    assert_eq!(game.left().unwrap(), 3, "read sees the journal");
    let total = game.add(5).unwrap();
    assert_eq!(game.left().unwrap(), 8, "thunk sees the applied journal");
    raw.set_message(Box::new(format!("proof{total}"))).unwrap();
    raw.set_cancel(true).unwrap();
}

mod_internals::raw_listeners!(call);
