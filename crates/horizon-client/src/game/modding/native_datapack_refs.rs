//! Datapack registry reads through a level handle.
use super::Game;
use mod_api::LevelRef;

impl Game<'_> {
    /// The datapack registry set of this server level's world session, when the level is the
    /// open one and this is a server.
    fn level_datapack_set(&self, level: LevelRef) -> Option<&mod_api::DatapackRegistrySet> {
        (self.is_logical_server()
            && self.native_lifecycle.epoch() == Some(level.epoch())
            && level.dimension() == self.native_level_key())
        .then_some(())?;
        self.mods.datapack_registries.as_ref()
    }

    /// The entry ids of a mod's datapack registry as the world last loaded it.
    pub(super) fn level_datapack_entries(
        &self,
        level: LevelRef,
        registry: &str,
    ) -> Option<Vec<String>> {
        let entries = self
            .level_datapack_set(level)?
            .entries(registry)
            .ok()?
            .into_iter()
            .map(str::to_owned)
            .collect();
        Some(entries)
    }

    /// A handle to one entry at the generation now loaded.
    pub(super) fn level_datapack_handle(
        &self,
        level: LevelRef,
        registry: &str,
        entry: &str,
    ) -> Result<mod_api::DatapackHandle, mod_api::DatapackReadError> {
        self.level_datapack_set(level)
            .ok_or(mod_api::DatapackReadError::UnknownRegistry)?
            .handle(registry, entry)
    }

    /// The fields of the entry a handle names, after the handle's session and generation check.
    pub(super) fn level_datapack_read(
        &self,
        level: LevelRef,
        handle: &mod_api::DatapackHandle,
    ) -> Result<Vec<(String, mod_api::DatapackValue)>, mod_api::DatapackReadError> {
        let set = self
            .level_datapack_set(level)
            .ok_or(mod_api::DatapackReadError::UnknownRegistry)?;
        Ok(set.get(handle)?.values.clone())
    }
}
