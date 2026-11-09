// hidden: a module of game-side types; mods use their re-exports
#[doc(hidden)]
pub mod behaviors;
pub use behaviors::{
    Activity, Behavior, BehaviorCx, BehaviorRefusal, BehaviorRegistrar, BrainMemory,
    BrainWalkTarget, MemoryStatus,
};
// hidden: the game's behavior plumbing, not part of the mod surface
#[doc(hidden)]
pub use behaviors::{BehaviorCxBackend, BehaviorRegistrarBackend, RegisteredBehavior};
