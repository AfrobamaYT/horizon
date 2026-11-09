//! The transaction journal fake: what a host keeps on each transaction it opens.

// no wire: in-process testing harness state, not a mod boundary payload.
/// A test host's transaction journal, as the game's keeps its storages: the state each open
/// transaction began with, put back when it aborts, and every close in order.
///
/// [`FakeHost`](super::FakeHost) keeps one over its players and level. A test host of its own
/// keeps one over its state: it returns what [`open`](Self::open) gives from `transaction_open`,
/// and puts back what [`close`](Self::close) returns from `transaction_close`. Like the game's,
/// it names each transaction by a token and refuses a close that does not name the innermost.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FakeJournal<S> {
    kept: Vec<(u64, S)>,
    closed: Vec<(usize, bool)>,
    next: u64,
}

impl<S> Default for FakeJournal<S> {
    fn default() -> Self {
        Self {
            kept: Vec::new(),
            closed: Vec::new(),
            next: 1,
        }
    }
}

impl<S> FakeJournal<S> {
    /// Opens a transaction, nested in the open one if any, that began with state `now`, and
    /// gives back the value naming it.
    pub fn open(&mut self, now: S) -> crate::Transaction {
        let token = self.next;
        self.next += 1;
        self.kept.push((token, now));
        crate::Transaction(token)
    }
    /// Closes the innermost open transaction when `tx` names it: committed, or aborted, when it
    /// gives back the state it began with for the host to put back. Nothing when none is open
    /// or `tx` names another.
    pub fn close(&mut self, tx: &crate::Transaction, commit: bool) -> Option<S> {
        if self.kept.last().map(|(token, _)| *token) != Some(tx.0) {
            return None;
        }
        let (_, was) = self.kept.pop()?;
        self.closed.push((self.kept.len() + 1, commit));
        (!commit).then_some(was)
    }
    /// Whether `tx` names an open transaction, for a write to check.
    pub fn holds(&self, tx: &crate::Transaction) -> bool {
        self.kept.iter().any(|(token, _)| *token == tx.0)
    }
    /// How many transactions are open.
    pub fn depth(&self) -> usize {
        self.kept.len()
    }
    /// Each transaction closed, in order: the depth it was open at (1 for the outermost), and
    /// whether it committed.
    pub fn closed(&self) -> &[(usize, bool)] {
        &self.closed
    }
}
