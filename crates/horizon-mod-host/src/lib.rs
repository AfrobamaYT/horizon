//! The mod host, above the engine (docs/layers.md, #2534). Empty until the moves of
//! docs/layering/mod-host-map.md land: loading first, then the `Host` implementations.
#![forbid(unsafe_code)]

mod loading;
pub use loading::main_with;
