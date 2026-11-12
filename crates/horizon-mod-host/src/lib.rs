//! Installed mod code loading above the engine (docs/layers.md, #2534).
//! The platform prepares the engine, loads mods here, then makes one engine start call.
#![forbid(unsafe_code)]

mod loading;
pub use loading::main_with;

pub use horizon_mod_wasm::compile_child;
/// Establish main-thread stack capacity before compiler-child handling.
pub use horizon_platform::prepare_main_stack;

/// Handles the private compiler invocation before any game startup work.
pub fn exit_if_compile_child() {
    horizon_platform::prepare_main_stack();
    if let Some(code) = compile_child(std::env::args_os().skip(1)) {
        std::process::exit(code);
    }
}
