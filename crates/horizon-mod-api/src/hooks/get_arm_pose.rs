//! The `get_arm_pose` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// NeoForge's `IClientItemExtensions.getArmPose`: the pose of the arm that holds `stack` in
    /// `hand`, as a humanoid model draws it (`AvatarRenderer.getArmPose`, for a player model
    /// the window's own or another player's). Set `pose` to one of vanilla's poses and it
    /// replaces what the item would give, whatever the use animation, a loaded crossbow or the
    /// item's own `arm_pose` declaration say; leave it unset to keep those. Asked once a
    /// tick per player model and non-empty hand, never for an empty hand. A two-handed
    /// pose (`bow_and_arrow`, `crossbow_hold`, `crossbow_charge`) still poses the other hand as
    /// vanilla's does. Vanilla's `spear` pose poses an arm only for an
    /// item with a kinetic weapon. NeoForge's custom poses made with enum extensions have no
    /// counterpart: the poses are vanilla's. Client-side.
    #[hook(
        side = "client",
        contract = "replace",
        out = "pose",
        neoforge = "IClientItemExtensions.getArmPose"
    )]
    fn get_arm_pose();
}
