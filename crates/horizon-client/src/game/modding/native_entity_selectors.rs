//! Mods' selector tokens (`@allies`, NeoForge's `EntitySelectorManager`): the base selections
//! the command registries parse them with, and their types' filter over the candidates.
use super::Game;
use super::commands::Target;
use horizon_client::command::args::Selector;
use horizon_client_core::registries::SelectorToken;
use mod_api::SelectorSort;

/// The tokens the installed mods registered, each with its type's base selection. A type whose
/// `base` panics is left out, with why.
pub(super) fn selector_tokens() -> Vec<SelectorToken> {
    mod_api::__private::entity_selector_types()
        .into_iter()
        .filter_map(|(name, selector)| {
            let Ok(base) =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| selector.base()))
            else {
                log::warn!("Horizon: selector token @{name} was left out: its base panicked");
                return None;
            };
            Some(SelectorToken {
                name,
                tooltip: base.tooltip,
                players_only: base.players_only,
                limit: base.limit.map_or(usize::MAX, |limit| limit as usize),
                sort: match base.sort {
                    SelectorSort::Nearest => "nearest",
                    SelectorSort::Furthest => "furthest",
                    SelectorSort::Random => "random",
                    _ => "arbitrary",
                },
            })
        })
        .collect()
}

impl Game<'_> {
    /// `select_in_level`, then, for a mod's token, only the candidates its type keeps: after the
    /// base selection and the `[...]` options, before sorting and the limit. A vanilla selector
    /// costs nothing more.
    pub(super) fn native_selector_kept(
        &mut self,
        s: &Selector,
        candidates: Vec<Target>,
        origin: [f64; 3],
    ) -> Vec<(Target, f64)> {
        let selected = self.select_in_level(candidates, s, origin);
        let Some(token) = &s.token else {
            return selected;
        };
        let Some(selector) = mod_api::__private::entity_selector_types()
            .into_iter()
            .find_map(|(name, selector)| (name == *token).then_some(selector))
        else {
            return Vec::new();
        };
        let Ok(source) = self.native_command_origin(self.ui.registries.permission) else {
            return Vec::new();
        };
        selected
            .into_iter()
            .filter(|(target, _)| {
                let Some(entity) = self.native_command_entity_target(*target) else {
                    return false;
                };
                let host: &mut dyn mod_api::ServerHost = self;
                // A panicking type keeps nothing, as an unanswered predicate.
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    selector.matches(host, &source, entity)
                }))
                .unwrap_or(false)
            })
            .collect()
    }
}
