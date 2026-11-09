//! `host.transaction` (#41 §7): NeoForge 26.3's `Transaction`. Changes made through the
//! capability helpers inside one are kept by the game and made final only when the outermost
//! transaction commits; an abort puts back everything its closure changed.
use super::ServerHost;

/// An open transaction, as NeoForge's `TransactionContext`: what every write of a handler or a
/// place takes. `host.transaction` gets one from the game and lends it to its closure alone.
///
/// The authority is the game's, not this value's: it names one transaction the game opened by an
/// unguessable token, and the game checks it on every use. A write given a transaction that is
/// not open changes nothing, and a close that does not name the innermost open transaction is
/// refused, so a value decoded from bytes or kept past its closure can neither write outside
/// the journal nor end a transaction someone else opened.
// exhaustive: its field is private, so mods cannot build or destructure it
#[derive(Debug, PartialEq, Eq)]
pub struct Transaction(pub(crate) u64);

/// The transaction the game opened with frame `token`, for its raw open helper.
#[doc(hidden)]
pub fn transaction(token: u64) -> Transaction {
    Transaction(token)
}

/// The frame token `tx` names, for the game's checks.
#[doc(hidden)]
pub fn transaction_token(tx: &Transaction) -> u64 {
    tx.0
}

/// Transactions are available only on the logical server and preserve the receiver type.
///
/// ```
/// use mod_api::{BlockEntityError, BlockEntityRef, HostTransaction, ServerHost};
/// fn write(host: &mut dyn ServerHost, block: BlockEntityRef) {
///     let _: Result<(), BlockEntityError> = host.transaction(|host, _tx| {
///         block.set_changed(host)?;
///         Ok(())
///     });
/// }
/// ```
/// ```compile_fail
/// use mod_api::{BlockEntityError, BlockEntityRef, Host, HostTransaction};
/// fn write(host: &mut dyn Host, block: BlockEntityRef) {
///     let _: Result<(), BlockEntityError> = host.transaction(|host, _tx| {
///         block.set_changed(host)?;
///         Ok(())
///     });
/// }
/// ```
/// ```compile_fail
/// use mod_api::{ClientHost, HostTransaction};
/// fn client(host: &mut dyn ClientHost) {
///     let _: Result<(), ()> = host.transaction(|_, _| Ok(()));
/// }
/// ```
pub trait HostTransaction: ServerHost {
    /// Runs `f` in a transaction, nested in the open one if any (`Transaction.openRoot` and
    /// `open`): `Ok` commits it (`commit`), into the transaction around it or, at the
    /// outermost, for good, with the world told of each change once; `Err` aborts it
    /// (`close` without `commit`), putting every storage it wrote back as it was. A panic
    /// unwinding out of `f` aborts this transaction as it passes, so whoever catches it sees
    /// nothing `f` changed, and the transaction around it stays open.
    ///
    /// Covered: block entities (item slots, saved compound, custom name and attachments), cauldrons,
    /// players' inventories and cursors, entities' inventories
    /// ([`ItemPlace::EntitySlot`](super::ItemPlace::EntitySlot)) and item entities' stacks.
    fn transaction<T, E>(
        &mut self,
        f: impl FnOnce(&mut Self, &Transaction) -> Result<T, E>,
    ) -> Result<T, E>;
}

impl<H: ServerHost + ?Sized> HostTransaction for H {
    fn transaction<T, E>(
        &mut self,
        f: impl FnOnce(&mut Self, &Transaction) -> Result<T, E>,
    ) -> Result<T, E> {
        run_transaction(self, f)
    }
}

/// `host.transaction` for a host of a known type, which `f` gets back: the game's own moves (a
/// hopper's) run in one through this.
pub fn run_transaction<H: ServerHost + ?Sized, T, E>(
    host: &mut H,
    f: impl FnOnce(&mut H, &Transaction) -> Result<T, E>,
) -> Result<T, E> {
    /// Closes its transaction when dropped: committed when `f` gave `Ok`, else aborted, as on
    /// a panic unwinding past it.
    struct Open<'a, H: ServerHost + ?Sized> {
        host: &'a mut H,
        tx: Transaction,
        commit: bool,
    }
    impl<H: ServerHost + ?Sized> Drop for Open<'_, H> {
        fn drop(&mut self) {
            self.host.transaction_close(&self.tx, self.commit);
        }
    }
    let tx = host.transaction_open();
    let mut open = Open {
        host,
        tx,
        commit: false,
    };
    let result = f(&mut *open.host, &open.tx);
    open.commit = result.is_ok();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeHost;

    #[test]
    fn a_transaction_decoded_from_bytes_ends_no_transaction() {
        use crate::prelude::ModStateServerHost as _;
        assert!(
            crate::wire::Reader::decode::<Transaction>(&[]).is_err(),
            "no transaction from zero bytes"
        );
        let forged: Transaction = crate::wire::Reader::decode(&1u64.to_le_bytes()).unwrap();
        let mut fake = FakeHost::new();
        let host: &mut dyn ServerHost = &mut fake;
        // Outside any transaction, and inside one the forged value does not name.
        host.transaction_close(&forged, true);
        let outer = host.transaction(|host, _| {
            host.transaction(|host, _| {
                host.transaction_close(&forged, true);
                Ok::<_, ()>(())
            })?;
            Err(())
        });
        assert_eq!(outer, Err(()));
        assert_eq!(
            fake.journal.closed(),
            [(2, true), (1, false)],
            "only each guard closed its own"
        );
        assert_eq!(fake.journal.depth(), 0);
    }

    #[test]
    fn a_transaction_commits_on_ok_and_aborts_on_err_and_nests() {
        let mut fake = FakeHost::new();
        let host: &mut dyn ServerHost = &mut fake;
        let outer = host.transaction(|host, _| {
            let inner: Result<(), &str> = host.transaction(|_, _| Err("refused"));
            assert_eq!(inner, Err("refused"));
            host.transaction(|_, _| Ok::<_, ()>(7))
        });
        assert_eq!(outer, Ok(7));
        assert_eq!(fake.journal.closed(), [(2, false), (2, true), (1, true)]);
        assert_eq!(fake.journal.depth(), 0);
    }

    #[test]
    fn a_panic_aborts_exactly_the_transaction_it_leaves() {
        use std::panic::{AssertUnwindSafe, catch_unwind};
        let mut fake = FakeHost::new();
        let host: &mut dyn ServerHost = &mut fake;
        // A child's panic caught inside the root: the child aborts, the root still commits.
        let root = host.transaction(|host, _| {
            let child = catch_unwind(AssertUnwindSafe(|| {
                host.transaction(|_, _| -> Result<(), ()> { panic!("a broken handler") })
            }));
            assert!(child.is_err());
            Ok::<_, ()>(())
        });
        assert_eq!(root, Ok(()));
        // A root's panic caught outside it: the root aborts before the catch sees anything.
        let host: &mut dyn ServerHost = &mut fake;
        let caught = catch_unwind(AssertUnwindSafe(|| {
            host.transaction(|_, _| -> Result<(), ()> { panic!("a broken handler") })
        }));
        assert!(caught.is_err());
        assert_eq!(fake.journal.closed(), [(2, false), (1, true), (1, false)]);
        assert_eq!(fake.journal.depth(), 0);
    }
}
