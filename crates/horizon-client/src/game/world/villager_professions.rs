//! Profession preparation participates in the existing WorldTables root publication.
use super::*;
use mod_api::__private::game::RegistrationContextNew;
impl Game<'_> {
    pub(super) fn prepare_professions(
        &self,
        root: &std::path::Path,
        items: &Items,
        stations: &mut horizon_client::stations::StationData,
    ) -> Result<()> {
        let declarations = self.mods.profession_declarations.get_or_init(|| {
            let mut registrar = mod_api::ProfessionDeclarations::default();
            Self::native_register_villager_professions(
                &mod_api::RegistrationContext::new(),
                &mut registrar,
            );
            registrar.into_entries()
        });
        let mut pois = horizon_world::poi::PoiSnapshot::default();
        for (_, definition) in declarations {
            for workstation in &definition.workstations {
                let name = workstation.block.to_string();
                if self.pack.assets.registry.default_state(&name).is_none() {
                    return Err(format!("unknown workstation block {name}").into());
                }
                let states: Vec<_> = (0..self.pack.assets.registry.len())
                    .filter_map(|index| {
                        let state = self
                            .pack
                            .assets
                            .registry
                            .state(horizon_world::registry::BlockStateId(index as u32))?;
                        (state.name == name
                            && workstation
                                .properties
                                .iter()
                                .all(|(key, value)| state.properties.get(key) == Some(value)))
                        .then(|| state.clone())
                    })
                    .collect();
                if states.is_empty() {
                    return Err(format!("workstation {name} selects no valid block states").into());
                }
                pois.add(&workstation.poi.0.to_string(), states)?;
            }
        }
        let mut candidate =
            horizon_client::mobs::villager::ResolvedProfessions::prepare(
                declarations.clone(),
                |kind, id| {
                    let name = id.to_string();
                    match kind {
                        "poi" => pois.resolve(&name).is_some(),
                        "trade_set" => stations.trades.sets.contains_key(&name),
                        "item" => items.get(&name).is_some(),
                        "block" => self.pack.assets.registry.default_state(&name).is_some(),
                        "sound" => std::fs::read(
                            root.join("assets").join(id.namespace()).join("sounds.json"),
                        )
                        .ok()
                        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                        .is_some_and(|events| events.get(id.path()).is_some()),
                        _ => false,
                    }
                },
            )
            .map_err(|errors| format!("villager professions refused: {errors:?}"))?;
        if !declarations.is_empty() {
            candidate.workstation_pois = (0..self.pack.assets.registry.len())
                .map(|index| {
                    self.pack
                        .assets
                        .registry
                        .state(horizon_world::registry::BlockStateId(index as u32))
                        .and_then(|state| pois.custom(state))
                })
                .collect();
        }
        candidate.pois = pois;
        stations.professions = Arc::new(candidate);
        stations.trades.professions = stations.professions.clone();
        Ok(())
    }
}

#[cfg(test)]
#[path = "villager_professions/tests.rs"]
mod tests;

impl Game<'_> {
    pub(super) fn resolve_loaded_profession(&mut self, mob: &mut horizon_client::mobs::Mob) {
        self.stations().professions.resolve_loaded(mob);
        if let horizon_client::mobs::villager::ProfessionState::Unresolved(id) =
            &mob.extra.profession
            && self.missing_professions.insert(id.clone())
        {
            log::warn!(
                "Unknown villager profession {id} at {:?}; preserving its saved identity",
                mob.position
            );
        }
    }
}

impl Game<'_> {
    pub(super) fn install_profession_save_snapshots(&self) {
        let table = self.stations().professions.clone();
        self.save.install_profession_snapshot(table.clone());
        self.levels.install_profession_save_snapshots(table);
    }
}
