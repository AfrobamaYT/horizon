use crate::EntityRef;

/// Per-window targeting controls. Defaults preserve vanilla picking. Reach remains backed
/// by `minecraft:block_interaction_range` and `minecraft:entity_interaction_range`; a server
/// mod changes those through the player's living attribute handle, not these client controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, horizon_mod_api_macros::Wire)]
#[non_exhaustive]
pub struct ClientTargeting {
    /// Include source and flowing fluid shapes in the ordinary outline pick.
    pub pick_liquids: bool,
    /// Let entity picks and mod-selected attacks pass block outlines on this client.
    pub hit_through_walls: bool,
    /// Prefer this client-level entity for the crosshair and attacks; None restores the ray.
    /// Stale, self, unsupported, out-of-reach or occluded targets cannot be attacked.
    pub entity: Option<EntityRef>,
}
