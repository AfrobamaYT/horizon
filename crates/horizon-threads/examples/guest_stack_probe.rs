//! Run with a small shell soft stack limit to exercise main-thread re-exec.
fn main() {
    horizon_threads::guest_stack::prepare_main();
    horizon_threads::guest_stack::check_current().unwrap();
    println!(
        "main guest stack available; reexec={:?}",
        std::env::var_os("HORIZON_STACK_REEXEC")
    );
}
