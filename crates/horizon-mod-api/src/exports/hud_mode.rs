/// HUD and first-person hand visibility. Other hand-render restrictions still apply.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum HudMode {
    /// Show both HUD and hands.
    #[default]
    AllVisible,
    /// Hide the HUD while allowing the first-person hands and held items.
    NoHud,
    /// Hide both HUD and hands.
    AllHidden,
}
