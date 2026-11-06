//! Observe allocations on the calling thread, including canonical lifting.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static PEAK: Cell<Option<usize>> = const { Cell::new(None) };
}

struct Allocator;
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

fn observe(size: usize) {
    let _ = PEAK.try_with(|peak| {
        if let Some(previous) = peak.get() {
            peak.set(Some(previous.max(size)));
        }
    });
}

// SAFETY: every allocation and deallocation is delegated unchanged to System.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        observe(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        observe(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        observe(size);
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

pub(super) fn measure<T>(f: impl FnOnce() -> T) -> (T, usize) {
    struct Restore(Option<usize>);
    impl Drop for Restore {
        fn drop(&mut self) {
            PEAK.set(self.0);
        }
    }
    let _restore = Restore(PEAK.replace(Some(0)));
    let result = f();
    (result, PEAK.get().unwrap())
}
