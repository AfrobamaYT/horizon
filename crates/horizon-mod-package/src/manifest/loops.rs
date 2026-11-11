//! `[[native.loop]]`: a native mod's type in the slot of a hot loop the game publishes
//! (`mod_api::loops`). The launcher compiles one copy of the loop with the installed mods' types
//! and installs it through the loop's entry function. Native only: a wasm mod is refused by name.
use serde::{Deserialize, Serialize};
use std::fmt;

/// The priority of a row that names none; lower runs first, ties by mod id.
pub const DEFAULT_PRIORITY: i32 = 1000;

/// One `[[native.loop]]` row.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Loop {
    pub id: String,
    /// The published loop, as the game's `loops.json` names it (`block_entities.tick`).
    #[serde(rename = "loop")]
    pub name: String,
    /// The slot type in the mod's crate, written `crate::Type` (or `crate::module::Type`): it
    /// implements the loop's slot trait and `Default + 'static`.
    pub with: String,
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_priority() -> i32 {
    DEFAULT_PRIORITY
}

/// Why a package's loop rows are refused before anything is built; `id` is `package:row`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoopError {
    /// A wasm mod declared a loop row.
    Wasm { id: String },
    Invalid {
        id: String,
        field: &'static str,
        fix: &'static str,
    },
}

impl fmt::Display for LoopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wasm { id } => write!(
                f,
                "E_LOOP_WASM: {id}: [[native.loop]] compiles game code; only native mods can fill a loop"
            ),
            Self::Invalid { id, field, fix } => {
                write!(f, "E_LOOP_INVALID: {id}: invalid {field}; {fix}")
            }
        }
    }
}
impl std::error::Error for LoopError {}

impl Loop {
    /// The slot type's path below the mod's crate root (`Throttle` for `crate::Throttle`).
    pub fn with_path(&self) -> Option<&str> {
        let path = self.with.strip_prefix("crate::")?;
        path.split("::")
            .all(|segment| {
                let mut chars = segment.chars();
                chars
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
            })
            .then_some(path)
    }

    pub(super) fn check_all(entries: &[Self], mod_id: &str) -> Result<(), LoopError> {
        for entry in entries {
            let id = format!("{mod_id}:{}", entry.id);
            if entry.name.trim().is_empty() {
                return Err(LoopError::Invalid {
                    id,
                    field: "loop",
                    fix: "name a published loop of the game, such as block_entities.tick",
                });
            }
            if entry.with_path().is_none() {
                return Err(LoopError::Invalid {
                    id,
                    field: "with",
                    fix: "name the slot type in this mod's crate as crate::Type",
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::Manifest;

    const HEADER: &str =
        "id='sample'\nname='Sample'\nversion='1.0.0'\napi='0.2'\ncapabilities=['registries']\n";
    const ROW: &str =
        "[[native.loop]]\nid='slow'\nloop='block_entities.tick'\nwith='crate::Slow'\n";

    fn native() -> String {
        format!(
            "[native]\ngame='26.3'\nhorizon='{0}'\nmixins='{0}'\n",
            "a".repeat(40)
        )
    }

    #[test]
    fn a_row_reads_with_its_default_priority() {
        let manifest = Manifest::parse(&format!("{HEADER}{}{ROW}", native())).unwrap();
        let row = &manifest.native.unwrap().r#loop[0];
        assert_eq!(row.name, "block_entities.tick");
        assert_eq!(row.with_path(), Some("Slow"));
        assert_eq!(row.priority, 1000);
        let manifest = Manifest::parse(&format!("{HEADER}{}{ROW}priority=-5\n", native())).unwrap();
        assert_eq!(manifest.native.unwrap().r#loop[0].priority, -5);
    }

    #[test]
    fn a_wasm_mod_is_refused_by_name() {
        let wasm = format!(
            "[wasm]\nsha512='{}'\ntoolchain='nightly'\n",
            "a".repeat(128)
        );
        let error = Manifest::parse(&format!("{HEADER}{wasm}{}{ROW}", native())).unwrap_err();
        assert!(
            error.contains("E_LOOP_WASM: sample:slow: [[native.loop]] compiles game code"),
            "{error}"
        );
    }

    #[test]
    fn bad_rows_are_refused_without_panicking() {
        for (old, new, field) in [
            ("with='crate::Slow'", "with='Slow'", Some("with")),
            ("with='crate::Slow'", "with='crate::'", Some("with")),
            ("with='crate::Slow'", "with='crate::a b'", Some("with")),
            ("with='crate::Slow'", "with='other::Slow'", Some("with")),
            ("loop='block_entities.tick'", "loop=' '", Some("loop")),
            ("with='crate::Slow'", "with=3", None),
            ("with='crate::Slow'", "with='crate::Slow'\nextra=1", None),
            ("[[native.loop]]", "[native.loop]", None),
            ("loop='block_entities.tick'\n", "", None),
        ] {
            let error = Manifest::parse(&format!("{HEADER}{}{}", native(), ROW.replace(old, new)))
                .unwrap_err();
            if let Some(field) = field {
                assert!(
                    error.contains(&format!("E_LOOP_INVALID: sample:slow: invalid {field};")),
                    "{error}"
                );
            }
        }
    }

    #[test]
    fn loop_rows_share_the_native_id_space() {
        let mixin =
            "[[native.mixin]]\nid='slow'\ntarget='game::Game::f'\nkind='HEAD'\nhelper='src/h.rs'\n";
        let error = Manifest::parse(&format!("{HEADER}{}{ROW}{mixin}", native())).unwrap_err();
        assert!(error.contains("duplicate"), "{error}");
        let error = Manifest::parse(&format!("{HEADER}{}{ROW}{ROW}", native())).unwrap_err();
        assert!(error.contains("duplicate"), "{error}");
    }
}
