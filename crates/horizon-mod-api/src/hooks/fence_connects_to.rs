//! The `fence_connects_to` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `FenceBlock.connectsTo`: set `connected` to say whether the fence
    /// `state` at `pos` connects an arm toward `neighbour`, the state beside it,
    /// `direction` pointing from the fence to that neighbour and `face_solid`
    /// whether the neighbour's face toward the fence is sturdy. `connected` holds
    /// vanilla's answer (a solid face, a fence of the same kind or a gate across the
    /// arm): change it, to false as well as to true, when the fence is your own.
    /// Asked whenever the fence settles its shape or is placed, inside the level's
    /// update: answer from the arguments without touching the world. Only the fence block's own part is asked, so vanilla fences cost
    /// nothing. Authoritative servers only.
    #[hook(
        side = "server",
        contract = "collect",
        out = "connected",
        scope = "block",
        key = "state",
        target = "horizon_client::game::native_hooks::GameLevel<'_>::native_fence_connects_to"
    )]
    fn fence_connects_to();
}
