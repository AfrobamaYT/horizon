// Standalone execution of the production numeric index; registry fixtures resolve known ordinals.
extern crate self as horizon_world;
extern crate self as mod_api;
use std::sync::atomic::{AtomicUsize, Ordering};
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
struct Alloc;
unsafe impl std::alloc::GlobalAlloc for Alloc {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { std::alloc::System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOC: Alloc = Alloc;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockKey(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockState(BlockKey, u16);
impl BlockState {
    pub fn block(self) -> BlockKey {
        self.0
    }
    pub fn ordinal(self) -> u16 {
        self.1
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockStateError {
    Unavailable,
    UnknownBlock,
    UnknownState,
}
pub mod registry {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct BlockStateId(pub u32);
    pub struct BlockRegistry(pub Vec<Result<crate::BlockState, crate::BlockStateError>>);
    impl BlockRegistry {
        pub fn len(&self) -> usize {
            self.0.len()
        }
        pub fn state(
            &self,
            id: BlockStateId,
        ) -> Option<&Result<crate::BlockState, crate::BlockStateError>> {
            self.0.get(id.0 as usize)
        }
    }
}
mod boundary {
    pub trait TryIntoApi {
        type Api;
        type Error;
        fn try_into_api(&self) -> Result<Self::Api, Self::Error>;
    }
    pub trait TryIntoEngine<C: ?Sized> {
        type Engine;
        type Error;
        fn try_into_engine(&self, c: &C) -> Result<Self::Engine, Self::Error>;
    }
    impl TryIntoApi for Result<crate::BlockState, crate::BlockStateError> {
        type Api = crate::BlockState;
        type Error = crate::BlockStateError;
        fn try_into_api(&self) -> Self {
            *self
        }
    }
    #[path = "INDEX_PATH"]
    mod block_state;
    pub fn run() {
        use crate::{registry::*, *};
        let north = BlockState(BlockKey(7), 0);
        let east = BlockState(BlockKey(7), 1);
        block_state::bind(&BlockRegistry(vec![Ok(north), Ok(east)])).unwrap();
        assert_eq!(BlockStateId(1).try_into_api(), Ok(east));
        assert_eq!(north.try_into_engine(&()), Ok(BlockStateId(0)));
        assert_eq!(
            BlockState(BlockKey(9), 0).try_into_engine(&()),
            Err(BlockStateError::UnknownBlock)
        );
        assert_eq!(
            BlockState(BlockKey(7), 2).try_into_engine(&()),
            Err(BlockStateError::UnknownState)
        );
        block_state::bind(&BlockRegistry(vec![Ok(east), Ok(north)])).unwrap();
        assert_eq!(BlockStateId(0).try_into_api(), Ok(east));
        assert_eq!(east.try_into_engine(&()), Ok(BlockStateId(0)));
        assert_eq!(block_state::snapshot().unwrap().as_ref(), &[east, north]);
        std::thread::spawn(|| {
            assert_eq!(
                BlockStateId(0).try_into_api(),
                Err(BlockStateError::Unavailable)
            )
        })
        .join()
        .unwrap();
        // Named identity baseline and both production directions, with identical black-box use.
        for direction in ["identity", "forward", "reverse"] {
            let before = ALLOCATIONS.load(Ordering::Relaxed);
            let time = std::time::Instant::now();
            for _ in 0..1_000_000 {
                match direction {
                    "identity" => {
                        std::hint::black_box(std::hint::black_box(0u32));
                    }
                    "forward" => {
                        std::hint::black_box(
                            std::hint::black_box(BlockStateId(0))
                                .try_into_api()
                                .unwrap(),
                        );
                    }
                    _ => {
                        std::hint::black_box(
                            std::hint::black_box(east).try_into_engine(&()).unwrap(),
                        );
                    }
                }
            }
            let ns = time.elapsed().as_nanos();
            let allocations = ALLOCATIONS.load(Ordering::Relaxed) - before;
            assert_eq!(allocations, 0);
            println!(
                "numeric_index direction={direction} iterations=1000000 ns={ns} allocations={allocations}"
            );
        }
        assert!(
            block_state::bind(&BlockRegistry(vec![Err(BlockStateError::UnknownState)])).is_err()
        );
        assert_eq!(
            BlockStateId(0).try_into_api(),
            Err(BlockStateError::Unavailable)
        );
        assert_eq!(east.try_into_engine(&()), Err(BlockStateError::Unavailable));
    }
}
fn main() {
    boundary::run();
}
