//! Thread-local counting, disabled except inside the measured host or guest region.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static COUNTS: Cell<(usize, usize, usize)> = const { Cell::new((0, 0, 0)) };
}
struct Counting;
#[global_allocator]
static ALLOCATOR: Counting = Counting;
fn record(bytes: usize, realloc: bool) {
    if ACTIVE.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNTS.try_with(|n| {
            let (a, r, b) = n.get();
            n.set((
                a + usize::from(!realloc),
                r + usize::from(realloc),
                b + bytes,
            ));
        });
    }
}
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size(), false);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size(), false);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size, true);
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
pub fn start(active: bool) {
    COUNTS.set((0, 0, 0));
    ACTIVE.set(active);
}
pub fn stop() -> (usize, usize, usize) {
    ACTIVE.set(false);
    COUNTS.get()
}
