//! Uses two declared fields and a thunk; the host observes only committed writes.
use mod_internals::view::{Game, HandleChatInputRaw};

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {}

// The name also verifies that runtime imports do not shadow a raw listener.
fn call(raw: HandleChatInputRaw, game: Option<Game>) {
    let game = game.expect("a method receiver");
    let message = raw.message().unwrap();
    if message.starts_with('x') {
        use mod_internals::runtime::{AsNode, bindings::horizon::mod_internals::view::Cell};
        // Lower once. The WIT setter borrows these cells, with no per-iteration string clone.
        let small = message.len() == 1;
        let cells = [Cell::Str(*message)];
        let node = game.node();
        if small {
            for _ in 0..2000 {
                assert!(node.set(0, &cells).is_err());
            }
            return;
        }
        loop {
            // `left` is i64: refusal precedes retention, so journal budgets cannot stop this.
            assert!(node.set(0, &cells).is_err());
        }
    }
    if matches!(message.as_str(), "full_journal" | "empty_journal") {
        use mod_internals::runtime::AsNode;
        if message.as_str() == "full_journal" {
            for _ in 0..4096 {
                game.set_left(42).unwrap();
            }
            loop {
                // Miss all pending writes, rather than finding the most recent left write.
                let _ = game.node().get(1).unwrap();
            }
        }
        for _ in 0..2000 {
            let _ = game.node().get(1).unwrap();
        }
        return;
    }
    match message.as_str() {
        "panic_thunk" => {
            game.set_left(42).unwrap();
            let _ = game.add(-999);
            game.set_right(999).unwrap();
            return;
        }
        "panic_apply" => {
            game.set_left(-777).unwrap();
            return;
        }
        "budget" => {
            for _ in 0..4097 {
                game.set_left(42).unwrap();
            }
            return;
        }
        _ => {}
    }
    game.set_left(game.left().unwrap() + 2).unwrap();
    game.set_right(game.right().unwrap() + 3).unwrap();
    assert_eq!(game.left().unwrap(), 3, "read sees the journal");
    let total = game.add(5).unwrap();
    assert_eq!(game.left().unwrap(), 8, "thunk sees the applied journal");
    raw.set_message(Box::new(format!("proof{total}"))).unwrap();
    raw.set_cancel(true).unwrap();
}

mod_internals::raw_listeners!(call);
