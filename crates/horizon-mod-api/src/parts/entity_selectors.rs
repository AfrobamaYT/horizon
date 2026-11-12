//! Concrete authoring of entity selector tokens: the checks `register_entity_selector` refuses with.
use super::*;

/// Vanilla's selector letters, which a mod's token never replaces.
const VANILLA: [&str; 6] = ["p", "a", "r", "s", "e", "n"];

impl EntitySelectorsRegistrar for PartRegistrar {
    fn add_entity_selector(&mut self, name: &str, selector: Arc<dyn crate::EntitySelectorType>) {
        let refusal = if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            Some(format!(
                "{} registered invalid selector token @{name}",
                self.mod_id
            ))
        } else if VANILLA.contains(&name) {
            Some(format!(
                "{} registered vanilla's selector @{name}",
                self.mod_id
            ))
        } else if self
            .parts
            .entity_selectors
            .iter()
            .any(|other| *other.name == *name)
        {
            Some(format!(
                "{} registered selector token @{name} twice",
                self.mod_id
            ))
        } else {
            None
        };
        if let Some(refusal) = refusal {
            self.refused.push(refusal);
            return;
        }
        self.parts.entity_selectors.push(EntitySelectorInfo {
            name: name.into(),
            selector,
        });
    }
}

/// Fixtures that tests of other crates use to install entity selector tokens.
#[cfg(any(test, feature = "testing"))]
pub mod tests {
    #[test]
    pub(crate) fn registration_refuses_vanilla_invalid_and_duplicate_tokens() {
        /// A token that keeps every candidate.
        struct Everyone;

        impl crate::EntitySelectorType for Everyone {
            fn base(&self) -> crate::SelectorBase {
                crate::SelectorBase::new(
                    true,
                    Some(1),
                    crate::SelectorSort::Nearest,
                    "test.selector.allies",
                )
            }

            fn matches(
                &self,
                _: &mut dyn crate::ServerHost,
                _: &crate::CommandSource,
                _: crate::EntityRef,
            ) -> bool {
                true
            }
        }

        struct Allies;
        #[mod_api::__subscriptions]
        impl crate::Hooks for Allies {
            fn register(&self, r: &mut dyn crate::Registrar, _: &crate::RegistrationContext) {
                r.register_entity_selector("allies", Everyone);
            }
        }

        struct Invalid;
        #[mod_api::__subscriptions]
        impl crate::Hooks for Invalid {
            fn register(&self, r: &mut dyn crate::Registrar, _: &crate::RegistrationContext) {
                r.register_entity_selector("allies", Everyone);
                r.register_entity_selector("allies", Everyone);
                r.register_entity_selector("a", Everyone);
                r.register_entity_selector("Bad", Everyone);
            }
        }

        crate::parts::tests::install_parts_fixture(&[crate::ModEntry::new(
            "test",
            Box::new(Allies),
        )])
        .unwrap();
        let installed = mod_vocab::__private::parts_installation::testing_entity_selectors();
        assert_eq!(installed.len(), 1);
        assert_eq!(&*installed[0].name, "allies");
        assert_eq!(installed[0].selector.base().limit, Some(1));

        let errors = crate::parts::tests::install_parts_fixture(&[crate::ModEntry::new(
            "test",
            Box::new(Invalid),
        )])
        .unwrap_err();
        assert_eq!(errors.len(), 3);
        assert!(errors[0].contains("@allies twice"));
        assert!(errors[1].contains("vanilla's selector @a"));
        assert!(errors[2].contains("invalid selector token @Bad"));
    }
}
