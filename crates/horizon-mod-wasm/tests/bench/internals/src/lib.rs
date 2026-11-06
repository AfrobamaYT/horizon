//! Each raw call performs N accesses; the host times and verifies the committed result.
use mod_internals::view::{Game, HandleChatInputRaw};

#[derive(Default)]
pub struct Mod;
#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {}

fn access(raw: HandleChatInputRaw, game: Option<Game>) {
    let game = game.unwrap();
    let message = raw.message().unwrap();
    let (mode, count) = message.split_once(' ').unwrap();
    let count: i64 = count.parse().unwrap();
    if mode == "trap" {
        game.set_copy(99).unwrap();
        panic!("discard this listener's pending write");
    }
    let mut total = 0;
    for i in 0..count {
        match mode {
            "get" => total += game.copy().unwrap(),
            "handle" => total += game.child().unwrap().value().unwrap(),
            "set" => game.set_copy(i).unwrap(),
            "call" => total += game.add(1).unwrap(),
            _ => unreachable!(),
        }
    }
    raw.set_message(Box::new(total.to_string())).unwrap();
}
mod_internals::raw_listeners!(access);
