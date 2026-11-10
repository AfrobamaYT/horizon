//! Vanilla point-of-interest classification for saved block states, and the
//! states mods add to a vanilla type (`extend_poi_types`).
use crate::chunk::BlockState;
use std::{collections::BTreeMap, sync::RwLock};

/// A vanilla point-of-interest identity, independent of villager professions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoiType {
    Armorer,
    Butcher,
    Cartographer,
    Cleric,
    Farmer,
    Fisherman,
    Fletcher,
    Leatherworker,
    Librarian,
    Mason,
    Shepherd,
    Toolsmith,
    Weaponsmith,
    Meeting,
    Home,
    Beehive,
    BeeNest,
    NetherPortal,
    Lodestone,
    TestInstance,
    LightningRod,
}

impl PoiType {
    /// Every vanilla type, in registration order.
    const ALL: [Self; 21] = [
        Self::Armorer,
        Self::Butcher,
        Self::Cartographer,
        Self::Cleric,
        Self::Farmer,
        Self::Fisherman,
        Self::Fletcher,
        Self::Leatherworker,
        Self::Librarian,
        Self::Mason,
        Self::Shepherd,
        Self::Toolsmith,
        Self::Weaponsmith,
        Self::Meeting,
        Self::Home,
        Self::Beehive,
        Self::BeeNest,
        Self::NetherPortal,
        Self::Lodestone,
        Self::TestInstance,
        Self::LightningRod,
    ];

    /// The type with this saved registry identifier, if vanilla has one.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|poi| poi.id() == id)
    }

    /// Saved registry identifier.
    pub fn id(self) -> &'static str {
        match self {
            Self::Armorer => "minecraft:armorer",
            Self::Butcher => "minecraft:butcher",
            Self::Cartographer => "minecraft:cartographer",
            Self::Cleric => "minecraft:cleric",
            Self::Farmer => "minecraft:farmer",
            Self::Fisherman => "minecraft:fisherman",
            Self::Fletcher => "minecraft:fletcher",
            Self::Leatherworker => "minecraft:leatherworker",
            Self::Librarian => "minecraft:librarian",
            Self::Mason => "minecraft:mason",
            Self::Shepherd => "minecraft:shepherd",
            Self::Toolsmith => "minecraft:toolsmith",
            Self::Weaponsmith => "minecraft:weaponsmith",
            Self::Meeting => "minecraft:meeting",
            Self::Home => "minecraft:home",
            Self::Beehive => "minecraft:beehive",
            Self::BeeNest => "minecraft:bee_nest",
            Self::NetherPortal => "minecraft:nether_portal",
            Self::Lodestone => "minecraft:lodestone",
            Self::TestInstance => "minecraft:test_instance",
            Self::LightningRod => "minecraft:lightning_rod",
        }
    }
    /// The compact ordinal of a vanilla village job site, if this is one.
    pub fn job_index(self) -> Option<usize> {
        self.is_job().then_some(self as usize)
    }

    /// Whether this is one of the vanilla village job sites.
    pub fn is_job(self) -> bool {
        matches!(
            self,
            Self::Armorer
                | Self::Butcher
                | Self::Cartographer
                | Self::Cleric
                | Self::Farmer
                | Self::Fisherman
                | Self::Fletcher
                | Self::Leatherworker
                | Self::Librarian
                | Self::Mason
                | Self::Shepherd
                | Self::Toolsmith
                | Self::Weaponsmith
        )
    }
    /// `PoiType.maxTickets`.
    pub fn max_tickets(self) -> usize {
        match self {
            Self::Meeting => 32,
            Self::Beehive
            | Self::BeeNest
            | Self::NetherPortal
            | Self::Lodestone
            | Self::TestInstance
            | Self::LightningRod => 0,
            _ => 1,
        }
    }
    /// `PoiType.validRange` for the village POIs.
    pub fn valid_range(self) -> i32 {
        if self == Self::Meeting { 6 } else { 1 }
    }
}

/// `PoiTypes.register`: saved POI type of a block state, vanilla's first, then
/// the one a mod gave it ([`extend`]).
pub fn saved_poi(state: &BlockState) -> Option<PoiType> {
    vanilla_poi(state).or_else(|| extension(state))
}

/// The POI type vanilla registers a block state under.
fn vanilla_poi(state: &BlockState) -> Option<PoiType> {
    use PoiType::*;
    let name = state.name.strip_prefix("minecraft:")?;
    Some(match name {
        "blast_furnace" => Armorer,
        "smoker" => Butcher,
        "cartography_table" => Cartographer,
        "brewing_stand" => Cleric,
        "composter" => Farmer,
        "barrel" => Fisherman,
        "fletching_table" => Fletcher,
        "cauldron" | "lava_cauldron" | "water_cauldron" | "powder_snow_cauldron" => Leatherworker,
        "lectern" => Librarian,
        "stonecutter" => Mason,
        "loom" => Shepherd,
        "smithing_table" => Toolsmith,
        "grindstone" => Weaponsmith,
        "bell" => Meeting,
        "beehive" => Beehive,
        "bee_nest" => BeeNest,
        "nether_portal" => NetherPortal,
        "lodestone" => Lodestone,
        "test_instance_block" => TestInstance,
        bed if bed.ends_with("_bed") && bed != "straw_bed" => {
            if state.properties.get("part").map(String::as_str) != Some("head") {
                return None;
            }
            Home
        }
        rod if rod.ends_with("lightning_rod") => LightningRod,
        _ => return None,
    })
}

/// Saved POI type and maximum tickets, for save and locate consumers.
pub fn poi_type(state: &BlockState) -> Option<(&'static str, i32)> {
    saved_poi(state).map(|poi| (poi.id(), poi.max_tickets() as i32))
}

/// A block state a mod adds to a vanilla POI type (NeoForge's `ExtendPoiTypesEvent`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoiExtension {
    /// The complete block state.
    pub state: BlockState,
    /// The saved identifier of the type asked for (`minecraft:home`).
    pub poi: String,
}

/// The states mods added, replaced as a whole each time the hook is collected.
static EXTENSIONS: RwLock<BTreeMap<BlockState, PoiType>> = RwLock::new(BTreeMap::new());

/// Replace the mods' additions with `additions`, in their order. Returns the
/// refusals, one message each: an unknown POI type, and a state vanilla or an
/// earlier addition already gives to a different type (the message names the
/// state and both types). Adding a state to the type it already has is a no-op.
pub fn extend(additions: Vec<PoiExtension>) -> Vec<String> {
    let mut table = BTreeMap::new();
    let mut refusals = Vec::new();
    for addition in additions {
        let Some(asked) = PoiType::from_id(&addition.poi) else {
            refusals.push(format!(
                "unknown POI type {} asked for {}",
                addition.poi,
                describe(&addition.state)
            ));
            continue;
        };
        let owner = vanilla_poi(&addition.state).or_else(|| table.get(&addition.state).copied());
        match owner {
            Some(owner) if owner != asked => refusals.push(format!(
                "{} is already a {} point of interest, not added to {}",
                describe(&addition.state),
                owner.id(),
                asked.id()
            )),
            Some(_) => {}
            None => {
                table.insert(addition.state, asked);
            }
        }
    }
    *EXTENSIONS
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = table;
    refusals
}

/// The POI type a mod gave a block state ([`extend`]), if any.
pub fn extension(state: &BlockState) -> Option<PoiType> {
    EXTENSIONS
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(state)
        .copied()
}

/// A state for a message: `minecraft:bell[attachment=floor,facing=north]`.
fn describe(state: &BlockState) -> String {
    if state.properties.is_empty() {
        return state.name.clone();
    }
    let properties: Vec<String> = state
        .properties
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    format!("{}[{}]", state.name, properties.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(name: &str, properties: &[(&str, &str)]) -> BlockState {
        BlockState {
            name: name.into(),
            properties: properties
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }

    /// The additions are one process-wide table: the tests that replace it run one at a time.
    static TABLE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn a_mod_adds_a_blocks_states_to_a_vanilla_type() {
        let _table = TABLE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let hay = state("minecraft:hay_block", &[("axis", "y")]);
        assert_eq!(saved_poi(&hay), None);
        let refusals = extend(
            ["x", "y", "z"]
                .into_iter()
                .map(|axis| PoiExtension {
                    state: state("minecraft:hay_block", &[("axis", axis)]),
                    poi: "minecraft:home".into(),
                })
                .collect(),
        );
        assert_eq!(refusals, Vec::<String>::new());
        assert_eq!(saved_poi(&hay), Some(PoiType::Home));
        assert_eq!(poi_type(&hay), Some(("minecraft:home", 1)));
        assert_eq!(extension(&hay), Some(PoiType::Home));
        // Vanilla's own states are unchanged, and a state the mod did not name stays none.
        assert_eq!(
            poi_type(&state("minecraft:bell", &[])),
            Some(("minecraft:meeting", 32))
        );
        assert_eq!(saved_poi(&state("minecraft:stone", &[])), None);
        // Collecting again replaces the table.
        assert_eq!(extend(Vec::new()), Vec::<String>::new());
        assert_eq!(saved_poi(&hay), None);
    }

    #[test]
    fn refusals_name_both_types_and_leave_vanilla_as_it_was() {
        let _table = TABLE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let bell = state("minecraft:bell", &[("attachment", "floor")]);
        let lectern = state("minecraft:lectern", &[("has_book", "false")]);
        let refusals = extend(vec![
            PoiExtension {
                state: bell.clone(),
                poi: "minecraft:home".into(),
            },
            PoiExtension {
                state: state("minecraft:stone", &[]),
                poi: "minecraft:nowhere".into(),
            },
            // The type it already has: a no-op, not a refusal.
            PoiExtension {
                state: lectern.clone(),
                poi: "minecraft:librarian".into(),
            },
            // An earlier addition owns the state.
            PoiExtension {
                state: state("minecraft:hay_block", &[("axis", "y")]),
                poi: "minecraft:home".into(),
            },
            PoiExtension {
                state: state("minecraft:hay_block", &[("axis", "y")]),
                poi: "minecraft:meeting".into(),
            },
            PoiExtension {
                state: state("minecraft:hay_block", &[("axis", "y")]),
                poi: "minecraft:home".into(),
            },
            // The rest of the additions still apply.
            PoiExtension {
                state: state("minecraft:hay_block", &[("axis", "x")]),
                poi: "minecraft:armorer".into(),
            },
        ]);
        assert_eq!(refusals.len(), 3, "{refusals:?}");
        assert!(
            refusals[0].contains("minecraft:bell[attachment=floor]")
                && refusals[0].contains("minecraft:meeting")
                && refusals[0].contains("minecraft:home"),
            "{}",
            refusals[0]
        );
        assert!(
            refusals[1].contains("minecraft:nowhere") && refusals[1].contains("minecraft:stone"),
            "{}",
            refusals[1]
        );
        assert!(
            refusals[2].contains("minecraft:hay_block[axis=y]")
                && refusals[2].contains("minecraft:home")
                && refusals[2].contains("minecraft:meeting"),
            "{}",
            refusals[2]
        );
        assert_eq!(saved_poi(&bell), Some(PoiType::Meeting));
        assert_eq!(saved_poi(&lectern), Some(PoiType::Librarian));
        assert_eq!(extension(&lectern), None);
        assert_eq!(
            saved_poi(&state("minecraft:hay_block", &[("axis", "y")])),
            Some(PoiType::Home)
        );
        assert_eq!(
            saved_poi(&state("minecraft:hay_block", &[("axis", "x")])),
            Some(PoiType::Armorer)
        );
        assert_eq!(saved_poi(&state("minecraft:stone", &[])), None);
        extend(Vec::new());
    }

    #[test]
    fn every_vanilla_type_is_found_by_its_id() {
        for poi in PoiType::ALL {
            assert_eq!(PoiType::from_id(poi.id()), Some(poi));
        }
        assert_eq!(PoiType::from_id("minecraft:nowhere"), None);
        assert_eq!(PoiType::from_id("home"), None);
    }

    #[test]
    fn saved_poi_types_include_all_cauldron_states() {
        for name in [
            "cauldron",
            "water_cauldron",
            "lava_cauldron",
            "powder_snow_cauldron",
        ] {
            let state = BlockState {
                name: format!("minecraft:{name}"),
                properties: Default::default(),
            };
            assert_eq!(poi_type(&state), Some(("minecraft:leatherworker", 1)));
        }
    }
}
