use super::*;

impl PartRegistrar {
    pub(crate) fn game_rule_id(&self, id: &str) -> Result<String, crate::GameRuleError> {
        let id = if id.contains(':') {
            id.to_owned()
        } else {
            format!("{}:{id}", self.mod_id)
        };
        if !horizon_mod_package::content::namespaced_id(&id) {
            return Err(crate::GameRuleError::InvalidId);
        }
        if id.split_once(':').unwrap().0 != self.mod_id {
            return Err(crate::GameRuleError::ForeignNamespace);
        }
        Ok(id)
    }
}

/// Construct a fresh registry from the immutable installed category snapshot.
// hidden: the game owns session registry construction.
#[doc(hidden)]
pub fn game_rule_registry() -> horizon_client_core::gamerules::RuleRegistry {
    let Some(parts) = installed() else {
        return horizon_client_core::gamerules::RuleRegistry::vanilla();
    };
    horizon_client_core::gamerules::RuleRegistry::with_bool_rules(
        &parts.game_rule_categories,
        &parts.game_rules,
    )
    .expect("validated at registration")
}
/// Dense position for a validated rule key.
// hidden: the game resolves load-owned keys to dense positions.
#[doc(hidden)]
pub fn game_rule_bool_slot(key: crate::BoolGameRuleKey) -> Option<u32> {
    installed()?.game_rule_slots.get(key.0 as usize).copied()
}
/// Fixtures that tests of other crates use to install Parts.
#[cfg(any(test, feature = "testing"))]
pub mod tests {
    use super::*;
    use crate::*;
    #[test]
    pub(crate) fn cross_mod_categories_resolve_after_all_registration() {
        use super::Registrar;
        struct Rule;
        #[mod_api::__subscriptions]
        impl Hooks for Rule {
            fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
                r.game_rule_bool("active", "later:machines", true).unwrap();
            }
        }
        struct Category;
        #[mod_api::__subscriptions]
        impl Hooks for Category {
            fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
                r.game_rule_category("machines").unwrap();
            }
        }
        let mods = [
            ModEntry::new("first", Box::new(Rule)),
            ModEntry::new("later", Box::new(Category)),
        ];
        super::install_parts(&mods).unwrap();
        assert!(super::game_rule_registry().rule("first:active").is_some());
    }
    #[test]
    pub(crate) fn unknown_category_reports_rule_category_and_owner_without_installing() {
        use super::Registrar;
        struct Rule;
        #[mod_api::__subscriptions]
        impl Hooks for Rule {
            fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
                r.game_rule_bool("active", "missing:machines", true)
                    .unwrap();
            }
        }
        let mods = [
            ModEntry::new("first", Box::new(Rule)),
            ModEntry::new("second", Box::new(Rule)),
        ];
        let errors = super::install_parts(&mods).unwrap_err();
        assert_eq!(errors.len(), 2);
        assert!(errors[0].contains("first:active"));
        assert!(errors[1].contains("second:active"));
        assert!(
            errors
                .iter()
                .all(|error| error.contains("missing:machines"))
        );
        assert!(super::game_rule_registry().rule("first:active").is_none());
    }

    #[test]
    pub(crate) fn bool_rule_declarations_validate_slots_and_fail_atomically() {
        use super::Registrar;
        let mut refused = super::PartRegistrar::new("example");
        refused
            .game_rule_bool("active", "missing:machines", true)
            .unwrap();
        let _ = refused.game_rule_bool("foreign:bad", "example:machines", true);
        let errors = super::install_registrar(refused).unwrap_err().join(" ");
        assert!(
            errors.contains("ForeignNamespace")
                && errors.contains("example:active")
                && errors.contains("missing:machines")
        );
        let mut good = super::PartRegistrar::new("example");
        let _key = good
            .game_rule_bool("active", "example:machines", true)
            .unwrap();
        good.game_rule_category("machines").unwrap();
        assert_eq!(
            good.game_rule_bool("active", "example:machines", true),
            Err(crate::GameRuleError::Duplicate)
        );
        assert!(super::install_registrar(good).is_err());
        let mut good = super::PartRegistrar::new("example");
        let key = good
            .game_rule_bool("active", "example:machines", true)
            .unwrap();
        good.game_rule_category("machines").unwrap();
        super::install_registrar(good).unwrap();
        assert!(crate::BoolGameRuleKey::from_slot(key.0).is_ok());
        assert!(crate::BoolGameRuleKey::from_slot(key.0 + 1).is_err());
        assert!(super::game_rule_bool_slot(key).is_some());
    }

    #[test]
    pub(crate) fn category_registration_refuses_ignored_errors_and_installs_snapshot() {
        use super::Registrar;
        let mut foreign = super::PartRegistrar::new("example");
        assert_eq!(
            foreign.game_rule_category("foreign:machines"),
            Err(crate::GameRuleError::ForeignNamespace)
        );
        assert!(super::install_registrar(foreign).is_err());
        let mut invalid = super::PartRegistrar::new("example");
        assert_eq!(
            invalid.game_rule_category("bad path"),
            Err(crate::GameRuleError::InvalidId)
        );
        assert!(super::install_registrar(invalid).is_err());
        let mut good = super::PartRegistrar::new("example");
        assert_eq!(good.game_rule_category("machines"), Ok(()));
        assert!(super::install_registrar(good).is_ok());
        assert!(
            super::game_rule_registry()
                .categories()
                .contains(&"example:machines".to_owned())
        );
        let mut duplicate = super::PartRegistrar::new("example");
        duplicate.game_rule_category("machines").unwrap();
        assert_eq!(
            duplicate.game_rule_category("example:machines"),
            Err(crate::GameRuleError::Duplicate)
        );
        assert!(super::install_registrar(duplicate).is_err());
    }
}
