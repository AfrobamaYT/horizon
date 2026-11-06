//! The `entity_data` area: host methods the game implements (`game/host/entity_data.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `entity_data` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait EntityDataBackend {
    /// Side: server.
    /// The entity's tags (`Entity.getTags`), as `/tag <entity> list` shows
    /// them. API 386.
    fn entity_tags(&self, entity: EntityRef) -> Option<Vec<String>> {
        crate::phase::unprovided(
            "Backend::entity_tags",
            "entity_tags",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The entity's name as plain text (`Entity.getName`): its custom name,
    /// a player's name, else its type's name. On a client (API
    /// @API:client_entity_refs@), from the custom name the server synced
    /// (`Entity.DATA_CUSTOM_NAME`) and a player's profile name. API
    /// 386.
    fn entity_name(&self, entity: EntityRef) -> Option<String> {
        crate::phase::unprovided(
            "Backend::entity_name",
            "entity_name",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the entity's name shows over it at any distance
    /// (`Entity.isCustomNameVisible`); false for the kinds that keep no name.
    fn entity_custom_name_visible(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_custom_name_visible",
            "entity_custom_name_visible",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The category of the entity's type (`EntityType.getCategory`): a mob's
    /// from its type, `Monster` for the dragon, `Misc` for every other kind.
    fn entity_classification(&self, entity: EntityRef) -> Option<MobCategory> {
        crate::phase::unprovided(
            "Backend::entity_classification",
            "entity_classification",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the entity is in its level: from when `entity_join_level` let
    /// it in (false while that hook runs for it and before) until it
    /// leaves or a mod removes it. Players, and the kinds that hook doesn't
    /// tell (displays, markers, interactions, evoker fangs, area effect
    /// clouds, ominous item spawners), are in it while the handle names them.
    /// False for a stale handle or on a client.
    fn entity_added_to_level(&self, entity: EntityRef) -> bool {
        crate::phase::unprovided(
            "Backend::entity_added_to_level",
            "entity_added_to_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `entity_data` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait EntityDataBackendMut {
    /// Side: server.
    /// The entity's data as `/data get entity` shows it
    /// (`Entity.saveWithoutId`): its compound without `id`. An entity with no
    /// UUID yet gets the one it will be saved with. API 386.
    fn entity_nbt(&mut self, entity: EntityRef) -> Option<Compound> {
        crate::phase::unprovided(
            "BackendMut::entity_nbt",
            "entity_nbt",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Merges `nbt` into the entity's data and loads the entity from it, as
    /// `/data merge entity` does (`EntityDataAccessor.setData`): true when it
    /// changed. False for a player (vanilla refuses them too), a kind Horizon
    /// can't reload yet (only mobs, dropped items, armor stands and item
    /// frames), data nested too deep, or data that changes nothing. API
    /// 386.
    fn entity_merge_nbt(&mut self, entity: EntityRef, nbt: Compound) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_merge_nbt",
            "entity_merge_nbt",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Adds tag `tag` (`Entity.addTag`): false when it has it already, holds
    /// 1024, its kind can't store tags yet, or the handle is stale. API 386.
    fn entity_add_tag(&mut self, entity: EntityRef, tag: &str) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_add_tag",
            "entity_add_tag",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Removes tag `tag` (`Entity.removeTag`): false when it didn't have it.
    /// API 386.
    fn entity_remove_tag(&mut self, entity: EntityRef, tag: &str) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_remove_tag",
            "entity_remove_tag",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Names the entity (`Entity.setCustomName`), or takes its name away
    /// with `None`: its name, its saved `CustomName` and, for a mob or an
    /// armor stand, what the players tracking it are sent. False for a
    /// player, a kind whose name Horizon keeps nowhere yet (projectiles, the
    /// dragon and crystals, primed TNT, falling blocks, experience orbs,
    /// ominous item spawners), a stale handle or on a client.
    fn entity_set_custom_name(&mut self, entity: EntityRef, name: Option<Text>) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_custom_name",
            "entity_set_custom_name",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Shows the entity's name at any distance, or only when looked at
    /// (`Entity.setCustomNameVisible`): saved, and sent to the players
    /// tracking a mob or an armor stand. False where `set_custom_name` is.
    fn entity_set_custom_name_visible(
        &mut self,
        entity: EntityRef,
        visible: bool,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_custom_name_visible",
            "entity_set_custom_name_visible",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
