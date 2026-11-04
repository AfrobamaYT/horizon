//! `HumanoidModel.ArmPose`: how a humanoid model poses an arm.

/// `HumanoidModel.ArmPose`'s constants, the arm poses a humanoid model has.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArmPose {
    Empty,
    Item,
    Block,
    Bow,
    CrossbowHold,
    CrossbowCharge,
    /// Raising a trident to throw it (or to launch with Riptide).
    ThrowTrident,
    /// Playing a goat horn.
    TootHorn,
    /// Looking through a spyglass (`HumanoidModel.ArmPose.SPYGLASS`).
    Spyglass,
    /// A spear held out (`HumanoidModel.ArmPose.SPEAR`), at rest or charging.
    Spear,
    /// A brush being used (`HumanoidModel.ArmPose.BRUSH`).
    Brush,
}

impl ArmPose {
    /// Every pose with the name an item declaration gives it.
    pub const ALL: [(&'static str, ArmPose); 11] = [
        ("empty", Self::Empty),
        ("item", Self::Item),
        ("block", Self::Block),
        ("bow_and_arrow", Self::Bow),
        ("crossbow_hold", Self::CrossbowHold),
        ("crossbow_charge", Self::CrossbowCharge),
        ("throw_trident", Self::ThrowTrident),
        ("toot_horn", Self::TootHorn),
        ("spyglass", Self::Spyglass),
        ("spear", Self::Spear),
        ("brush", Self::Brush),
    ];

    /// The pose an item declaration names, `None` for a name that is no pose.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find_map(|&(n, pose)| (n == name).then_some(pose))
    }

    /// Both hands hold the item (`BOW_AND_ARROW`'s and the crossbow's two).
    pub fn two_handed(self) -> bool {
        matches!(self, Self::Bow | Self::CrossbowHold | Self::CrossbowCharge)
    }

    /// The other hand's pose gives way to this one.
    pub fn affects_offhand(self) -> bool {
        self.two_handed() || self == Self::ThrowTrident
    }
}
