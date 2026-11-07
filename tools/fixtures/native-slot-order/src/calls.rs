// Identical callbacks for listed and source call records, sharing the function observers.
include!("functions.rs");
use internals::raw::{
    __S61fae50b74a9bcdbArgs as Args, __S61fae50b74a9bcdbOutput as Output,
    __S61fae50b74a9bcdbReceiver as Receiver,
};
static BEFORE: AtomicBool = AtomicBool::new(false);
static AFTER: AtomicBool = AtomicBool::new(false);
pub fn before(_: Receiver<'_, '_>, args: &mut Args) {
    assert_eq!(
        mod_api::crash::current_native_mod(),
        Some("native_slot_order")
    );
    BEFORE.store(true, Ordering::SeqCst);
    if args.rate == 1.0 {
        eprintln!("SLOT ORDER CALL BEFORE rate=1");
    }
}
pub fn after(_: Receiver<'_, '_>, value: &Output) {
    assert!(
        BEFORE.swap(false, Ordering::SeqCst),
        "before then original then after"
    );
    AFTER.store(true, Ordering::SeqCst);
    if *value == 300 {
        eprintln!("SLOT ORDER CALL AFTER value=300");
    }
}
pub fn call_value(_: Receiver<'_, '_>, value: Output) -> Output {
    assert!(
        AFTER.swap(false, Ordering::SeqCst),
        "after then modify value"
    );
    if value == 300 {
        eprintln!("SLOT ORDER CALL VALUE: 300 -> 319");
    }
    value + 19
}
