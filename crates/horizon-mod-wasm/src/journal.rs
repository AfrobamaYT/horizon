//! A call's writes to game internals (#589 W), held until they apply. A wasm listener's `set`s
//! land here, not in the game: its own reads see them, and they apply in order when the listener
//! returns, or before it calls a game function. A listener that traps leaves them unapplied, so
//! the game stays as it was before the call, or after its last game function.
//!
//! Each write names its place by path (a root of the call, then steps), never by pointer: the
//! path is resolved again when it applies, and a write whose path no longer leads anywhere (an
//! index past the end, a key gone, another enum case) is dropped and reported, not applied
//! somewhere else.

/// One step from a node to the next.
#[derive(Clone, Debug, PartialEq)]
pub enum Step<V> {
    /// A struct's field, or a case's, by its id.
    Field(u32),
    /// A sequence's element.
    Index(u64),
    /// A map's entry.
    Key(V),
}

/// Where a node is: one of the call's roots, then steps.
#[derive(Clone, Debug, PartialEq)]
pub struct Path<V> {
    pub root: u32,
    pub steps: Vec<Step<V>>,
}

/// A write the game couldn't apply: its path no longer leads anywhere.
#[derive(Debug, PartialEq)]
pub struct Gone;

/// The writes of one call, in order.
#[derive(Debug)]
pub struct Journal<V> {
    writes: Vec<(Path<V>, u32, V)>,
}

impl<V> Default for Journal<V> {
    fn default() -> Self {
        Self { writes: Vec::new() }
    }
}

impl<V: PartialEq> Journal<V> {
    /// Notes a write of field `field` of the node at `path`.
    pub fn set(&mut self, path: Path<V>, field: u32, value: V) {
        self.writes.push((path, field, value));
    }

    /// The latest value this call wrote to field `field` of the node at `path`, if it wrote one: a
    /// read sees the call's own writes before the game's.
    pub fn get(&self, path: &Path<V>, field: u32) -> Option<&V> {
        self.writes
            .iter()
            .rev()
            .find(|(p, f, _)| *f == field && p == path)
            .map(|(_, _, value)| value)
    }

    /// Applies every write, in the order it was made, through `write`, and empties the journal:
    /// when the listener returns, or before it calls a game function. Answers the writes whose
    /// path was gone, each with its field.
    pub fn apply(
        &mut self,
        mut write: impl FnMut(&Path<V>, u32, V) -> Result<(), Gone>,
    ) -> Vec<(Path<V>, u32)> {
        let mut gone = Vec::new();
        for (path, field, value) in self.writes.drain(..) {
            if write(&path, field, value).is_err() {
                gone.push((path, field));
            }
        }
        gone
    }

    /// Whether any write's path satisfies `under`.
    pub fn any(&self, mut under: impl FnMut(&Path<V>) -> bool) -> bool {
        self.writes.iter().any(|(path, _, _)| under(path))
    }

    /// Whether the call wrote nothing yet.
    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny game: players' health by index, and one flag.
    #[derive(Debug, PartialEq)]
    struct Game {
        health: Vec<i64>,
        flag: i64,
    }

    const HEALTH: u32 = 0;
    const FLAG: u32 = 1;

    fn player(i: u64) -> Path<i64> {
        Path {
            root: 0,
            steps: vec![Step::Index(i)],
        }
    }

    fn top() -> Path<i64> {
        Path {
            root: 0,
            steps: vec![],
        }
    }

    /// The game's side: resolve the path again, then write.
    fn write(game: &mut Game) -> impl FnMut(&Path<i64>, u32, i64) -> Result<(), Gone> + '_ {
        |path, field, value| match (&path.steps[..], field) {
            ([], FLAG) => {
                game.flag = value;
                Ok(())
            }
            ([Step::Index(i)], HEALTH) => {
                *game.health.get_mut(*i as usize).ok_or(Gone)? = value;
                Ok(())
            }
            _ => Err(Gone),
        }
    }

    #[test]
    fn a_call_reads_its_own_writes_and_the_game_sees_them_only_when_they_apply() {
        let mut game = Game {
            health: vec![20, 20],
            flag: 0,
        };
        let mut journal = Journal::default();
        journal.set(player(1), HEALTH, 5);
        journal.set(top(), FLAG, 1);
        journal.set(player(1), HEALTH, 7);
        assert_eq!(journal.get(&player(1), HEALTH), Some(&7));
        assert_eq!(journal.get(&player(0), HEALTH), None);
        assert_eq!(journal.get(&top(), HEALTH), None);
        // Nothing reached the game yet.
        assert_eq!(game.health, [20, 20]);
        assert!(journal.apply(write(&mut game)).is_empty());
        assert_eq!(
            game,
            Game {
                health: vec![20, 7],
                flag: 1
            }
        );
        assert!(journal.is_empty());
    }

    #[test]
    fn a_write_whose_path_is_gone_is_dropped_and_the_rest_still_apply_in_order() {
        let mut game = Game {
            health: vec![20, 20, 20],
            flag: 0,
        };
        let mut journal = Journal::default();
        journal.set(player(2), HEALTH, 1);
        journal.set(top(), FLAG, 3);
        // The listener's game function took a player away before the writes applied.
        game.health.pop();
        let gone = journal.apply(write(&mut game));
        assert_eq!(gone, [(player(2), HEALTH)]);
        assert_eq!(
            game,
            Game {
                health: vec![20, 20],
                flag: 3
            }
        );
    }
}
