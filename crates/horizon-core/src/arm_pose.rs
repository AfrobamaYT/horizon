//! `HumanoidModel.ArmPose`: how a humanoid model poses an arm, and the names an item declaration gives them.

/// `HumanoidModel.ArmPose`'s constants, the arm poses a humanoid model has.
// exhaustive: vanilla constants and a resolved client registration
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmPose {
    Empty,
    Item,
    Block,
    #[serde(rename = "bow_and_arrow")]
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
    /// A client registration resolved before animation. Rotations are IEEE bits;
    /// non-finite components preserve the base model angle.
    #[serde(skip)]
    Custom {
        slot: u32,
        both: bool,
        offhand: bool,
        angles: [[u32; 3]; 2],
    },
}

impl ArmPose {
    /// Both hands hold the item (`BOW_AND_ARROW`'s and the crossbow's two).
    pub fn two_handed(self) -> bool {
        matches!(
            self,
            Self::Bow | Self::CrossbowHold | Self::CrossbowCharge | Self::Custom { both: true, .. }
        )
    }

    /// The other hand's pose gives way to this one.
    pub fn affects_offhand(self) -> bool {
        match self {
            Self::Custom { offhand, .. } => offhand,
            _ => self.two_handed() || self == Self::ThrowTrident,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ArmPose;
    #[test]
    fn registered_pose_flags_remain_independent_in_the_engine() {
        let pose = ArmPose::Custom {
            slot: 0,
            both: true,
            offhand: false,
            angles: [[f32::NAN.to_bits(); 3]; 2],
        };
        assert!(pose.two_handed());
        assert!(!pose.affects_offhand());
        let pose = ArmPose::Custom {
            slot: 0,
            both: false,
            offhand: true,
            angles: [[0; 3]; 2],
        };
        assert!(!pose.two_handed());
        assert!(pose.affects_offhand());
    }
}
