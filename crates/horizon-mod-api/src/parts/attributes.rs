use super::*;
use horizon_world::attributes::Definition;
pub use horizon_world::attributes::Sentiment;

/// An attribute type a mod declares, as the game reads it when it grows the
/// attribute catalog (`Catalog::with_attributes`, #812).
#[derive(Clone, Debug)]
// exhaustive: only registration builds it; the game reads it
pub struct AttributeDeclaration {
    /// The declaring mod's id: the first key of the catalog's sort order.
    pub mod_id: String,
    /// The attribute's namespaced id (`mymod:mana`).
    pub id: String,
    /// The attribute's definition: default and bounds.
    pub definition: Definition,
    /// Whether the attribute joins the sync set.
    pub syncable: bool,
    /// The tooltip sentiment of a rising or falling value.
    pub sentiment: Sentiment,
}

/// The attribute types mods declared.
pub fn attribute_declarations() -> &'static [AttributeDeclaration] {
    installed().map_or(&[], |installed| &installed.attributes)
}

/// Registration of new attribute types ([`Registrar::declare_attribute`]).
pub trait AttributesRegistrar {
    /// Declare an attribute type of the mod's own (vanilla's
    /// `RangedAttribute` in `BuiltInRegistries.ATTRIBUTE`, NeoForge's
    /// `RegisterEvent`): `path` in the mod's namespace (`mana` of `mymod` is
    /// `mymod:mana`), its `default` and `[min, max]` bounds, whether it
    /// joins the attribute sync (`Attribute.setSyncable`), and the sentiment
    /// that colors its tooltip lines (`Attribute.setSentiment`, POSITIVE by
    /// default; the translation key is `attribute.name.<ns>.<path>`).
    ///
    /// The declarations join the attribute catalog once, at load, appended
    /// after every vanilla attribute **sorted by declaring mod id, then by
    /// attribute id** — the order is the network id's, so every side running
    /// the same mods builds the same numbers and vanilla ids never shift —
    /// before entity attribute rows resolve, so a mod entity's declared set
    /// may name the mod's own new attribute. Sanitizing a value (NaN → min,
    /// else clamp) is the definition's own `sanitize`.
    ///
    /// An id outside the mod's namespace (a vanilla attribute's included),
    /// a malformed id, a duplicate — of this mod's or another's, caught
    /// where the declarations join the catalog — a non-finite default or
    /// bound, or bounds the default is not inside (`min > max` included)
    /// refuses, naming the mod.
    fn declare_attribute(
        &mut self,
        path: &str,
        default: f64,
        min: f64,
        max: f64,
        syncable: bool,
        sentiment: Sentiment,
    ) -> Result<(), crate::AttributeRegistrationError>;
}

impl AttributesRegistrar for PartRegistrar {
    fn declare_attribute(
        &mut self,
        path: &str,
        default: f64,
        min: f64,
        max: f64,
        syncable: bool,
        sentiment: Sentiment,
    ) -> Result<(), crate::AttributeRegistrationError> {
        let result = (|| {
            let (namespace, name) = path.split_once(':').unwrap_or((self.mod_id, path));
            if namespace.is_empty() || name.is_empty() || namespace != self.mod_id {
                return Err(crate::AttributeRegistrationError::InvalidId);
            }
            let id = format!("{namespace}:{name}");
            // Identifier normalization is shared with modifier and catalog ids.
            if !horizon_world::attributes::Modifier::new(
                &id,
                0.0,
                horizon_world::attributes::Operation::AddValue,
            )
            .is_ok_and(|canonical| canonical.id() == id)
            {
                return Err(crate::AttributeRegistrationError::InvalidId);
            }
            if self.attributes.iter().any(|declared| declared.id == id) {
                return Err(crate::AttributeRegistrationError::Duplicate);
            }
            if ![default, min, max].into_iter().all(f64::is_finite) {
                return Err(crate::AttributeRegistrationError::NotFinite);
            }
            let definition = Definition::ranged(default, min, max)
                .map_err(|_| crate::AttributeRegistrationError::InvalidBounds)?;
            self.attributes.push(AttributeDeclaration {
                mod_id: self.mod_id.to_owned(),
                id,
                definition,
                syncable,
                sentiment,
            });
            Ok(())
        })();
        if let Err(error) = &result {
            self.refused
                .push(format!("mod {} attribute {path:?}: {error:?}", self.mod_id));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AttributeRegistrationError;
    pub use horizon_world::attributes::Sentiment;

    /// `RangedAttribute`'s own checks and the registration's: an id outside
    /// the mod's namespace (a vanilla attribute's included), a duplicate
    /// across mods, non-finite values and bounds the default is not inside
    /// all refuse, naming the mod; the good declaration stands.
    #[test]
    pub(crate) fn attribute_declarations_validate() {
        let mut registrar = PartRegistrar::new("mymod");
        registrar
            .declare_attribute("mana", 10.0, 0.0, 100.0, true, Sentiment::Neutral)
            .unwrap();
        // The path stays in the declaring mod's namespace.
        assert_eq!(
            registrar.declare_attribute("minecraft:armor", 0.0, 0.0, 30.0, true, positive()),
            Err(AttributeRegistrationError::InvalidId)
        );
        assert_eq!(
            registrar.declare_attribute("foreign:mana", 0.0, 0.0, 1.0, false, positive()),
            Err(AttributeRegistrationError::InvalidId)
        );
        assert_eq!(
            registrar.declare_attribute("Not An Id", 0.0, 0.0, 1.0, false, positive()),
            Err(AttributeRegistrationError::InvalidId)
        );
        // The same attribute twice, whatever declared it, is a duplicate.
        assert_eq!(
            registrar.declare_attribute("mana", 1.0, 0.0, 1.0, false, positive()),
            Err(AttributeRegistrationError::Duplicate)
        );
        // RangedAttribute's own bounds checks.
        assert_eq!(
            registrar.declare_attribute("rage", 5.0, 10.0, 0.0, false, positive()),
            Err(AttributeRegistrationError::InvalidBounds)
        );
        assert_eq!(
            registrar.declare_attribute("rage", 0.0, 1.0, 10.0, false, positive()),
            Err(AttributeRegistrationError::InvalidBounds)
        );
        assert_eq!(
            registrar.declare_attribute("rage", 20.0, 1.0, 10.0, false, positive()),
            Err(AttributeRegistrationError::InvalidBounds)
        );
        // A non-finite default or bound.
        assert_eq!(
            registrar.declare_attribute("rage", f64::NAN, 0.0, 1.0, false, positive()),
            Err(AttributeRegistrationError::NotFinite)
        );
        assert_eq!(
            registrar.declare_attribute("rage", 0.5, f64::NEG_INFINITY, 1.0, false, positive()),
            Err(AttributeRegistrationError::NotFinite)
        );
        assert_eq!(
            registrar.declare_attribute("rage", 0.5, 0.0, f64::INFINITY, false, positive()),
            Err(AttributeRegistrationError::NotFinite)
        );
        // The refusals name the mod; the good declaration stands.
        assert_eq!(registrar.attributes.len(), 1);
        let declared = &registrar.attributes[0];
        assert_eq!(
            (declared.mod_id.as_str(), declared.id.as_str()),
            ("mymod", "mymod:mana")
        );
        assert_eq!(declared.definition.default_value(), 10.0);
        assert_eq!(declared.definition.sanitize(-5.0), 0.0);
        assert_eq!(declared.definition.sanitize(500.0), 100.0);
        assert!(declared.syncable);
        assert_eq!(declared.sentiment, Sentiment::Neutral);
        assert!(
            registrar
                .refused
                .iter()
                .all(|refusal| refusal.contains("mod mymod"))
        );
    }

    fn positive() -> Sentiment {
        Sentiment::default()
    }
}
