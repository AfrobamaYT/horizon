//! `host.transaction` (#41 §7): NeoForge 26.3's `Transaction`. Changes made through the
//! capability helpers inside one are kept by the game and made final only when the outermost
//! transaction commits; an abort puts back everything its closure changed.
use super::Host;

impl dyn Host + '_ {
    /// Runs `f` in a transaction, nested in the open one if any (`Transaction.openRoot` and
    /// `open`): `Ok` commits it (`commit`), into the transaction around it or, at the
    /// outermost, for good, with the world told of each change once; `Err` aborts it
    /// (`close` without `commit`), putting every storage it wrote back as it was. A panic
    /// inside aborts every open transaction before the game's next tick.
    ///
    /// Covered: block entities (item slots, saved compound and attachments) and cauldrons.
    /// What an item entity's or a player's inventory does inside one is final at once.
    pub fn transaction<T, E>(
        &mut self,
        f: impl FnOnce(&mut dyn Host) -> Result<T, E>,
    ) -> Result<T, E> {
        self.transaction_open();
        let result = f(self);
        self.transaction_close(result.is_ok());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records the transactions it is asked to open and close.
    #[derive(Default)]
    struct Journal {
        depth: usize,
        closed: Vec<(usize, bool)>,
    }
    impl Host for Journal {
        fn transaction_open(&mut self) {
            self.depth += 1;
        }
        fn transaction_close(&mut self, commit: bool) {
            self.closed.push((self.depth, commit));
            self.depth -= 1;
        }
    }

    #[test]
    fn a_transaction_commits_on_ok_and_aborts_on_err_and_nests() {
        let mut journal = Journal::default();
        let host: &mut dyn Host = &mut journal;
        let outer = host.transaction(|host| {
            let inner: Result<(), &str> = host.transaction(|_| Err("refused"));
            assert_eq!(inner, Err("refused"));
            host.transaction(|_| Ok::<_, ()>(7))
        });
        assert_eq!(outer, Ok(7));
        assert_eq!(journal.closed, [(2, false), (2, true), (1, true)]);
        assert_eq!(journal.depth, 0);
    }
}
