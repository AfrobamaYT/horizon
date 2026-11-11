//! Load-scoped registration and per-candidate condition lookup.
use super::Game;
use mod_api::__private::game::RegistrationContextNew;
use std::{collections::BTreeMap, sync::Arc};
pub(super) type ConditionTable =
    Arc<BTreeMap<mod_api::Identifier, (String, Arc<dyn mod_api::LoadCondition>)>>;
impl Game<'_> {
    pub(super) fn begin_condition_reload(
        &mut self,
        stack: &super::functions::PackStack,
        resources: &std::path::Path,
    ) -> Result<
        super::server_reload::PreparedServerReload,
        mod_api::__private::reload::ReloadStageError,
    > {
        use mod_api::__private::reload::ReloadStageError;
        let inventory = horizon_mod_package::enabled::EnabledMods::read(resources)
            .map_err(|error| ReloadStageError::DataSource(error.to_string()))?;
        self.mods.load_condition_candidate = None;
        let candidate = self
            .registered_load_conditions(&inventory)
            .map_err(ReloadStageError::DataSource)?;
        self.mods.load_condition_candidate = Some(candidate);
        self.prepare_server_reload(stack, resources)
    }
    pub(super) fn registered_load_conditions(
        &self,
        inventory: &horizon_mod_package::enabled::EnabledMods,
    ) -> Result<horizon_world::conditions::LoadConditions, String> {
        if let Some(candidate) = &self.mods.load_condition_candidate {
            return Ok(candidate.clone());
        }
        let table = self
            .mods
            .load_conditions
            .get_or_init(|| {
                let mut registrar = mod_api::LoadConditionDeclarations::default();
                Self::native_register_load_conditions(
                    &mod_api::RegistrationContext::new(),
                    &mut registrar,
                );
                registrar
                    .finish()
                    .map(Arc::new)
                    .map_err(|error| error.message)
            })
            .clone()?;
        let conditions = horizon_world::conditions::LoadConditions::new(inventory.server_ids());
        if table.is_empty() {
            return Ok(conditions);
        }
        let owned = table
            .iter()
            .map(|(name, (owner, _))| (name.namespace().to_owned(), owner.clone()))
            .collect();
        Ok(conditions.with_lookup(owned, move |name, args| {
            let id = mod_api::Identifier::parse(name).ok()?;
            let (owner, handler) = table.get(&id)?;
            let mut arguments = args.as_object().cloned().unwrap_or_default();
            arguments.remove("type");
            Some(
                handler
                    .test(&mod_api::serde_json::Value::Object(arguments))
                    .map_err(|error| format!("condition {name} (mod {owner}): {}", error.message)),
            )
        }))
    }
}

#[cfg(test)]
#[path = "load_conditions/tests.rs"]
mod tests;
