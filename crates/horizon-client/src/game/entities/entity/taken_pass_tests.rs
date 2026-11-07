//! One-shot spawn injection while a production tick owns a lifted batch.
use crate::game::Game;
use std::cell::Cell;

type Spawn = fn(&mut Game<'_>);
thread_local! {
    static AFTER_TAKE: Cell<Option<Spawn>> = const { Cell::new(None) };
}

pub(in crate::game) fn with_spawn(spawn: Spawn, tick: impl FnOnce()) {
    assert!(AFTER_TAKE.replace(Some(spawn)).is_none());
    tick();
    assert!(AFTER_TAKE.take().is_none(), "tick did not lift its batch");
}

pub(in crate::game) fn after_take(game: &mut Game<'_>) {
    if let Some(spawn) = AFTER_TAKE.take() {
        spawn(game);
    }
}
