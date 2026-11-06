//! Vanilla point-of-interest classification for saved block states.

/// `PoiTypes.register`: the POI type of a block state and its max tickets.
pub fn poi_type(state: &crate::chunk::BlockState) -> Option<(&'static str, i32)> {
    let name = state.name.strip_prefix("minecraft:")?;
    Some(match name {
        "blast_furnace" => ("minecraft:armorer", 1),
        "smoker" => ("minecraft:butcher", 1),
        "cartography_table" => ("minecraft:cartographer", 1),
        "brewing_stand" => ("minecraft:cleric", 1),
        "composter" => ("minecraft:farmer", 1),
        "barrel" => ("minecraft:fisherman", 1),
        "fletching_table" => ("minecraft:fletcher", 1),
        "cauldron" | "lava_cauldron" | "water_cauldron" | "powder_snow_cauldron" => {
            ("minecraft:leatherworker", 1)
        }
        "lectern" => ("minecraft:librarian", 1),
        "stonecutter" => ("minecraft:mason", 1),
        "loom" => ("minecraft:shepherd", 1),
        "smithing_table" => ("minecraft:toolsmith", 1),
        "grindstone" => ("minecraft:weaponsmith", 1),
        "bell" => ("minecraft:meeting", 32),
        "beehive" => ("minecraft:beehive", 0),
        "bee_nest" => ("minecraft:bee_nest", 0),
        "nether_portal" => ("minecraft:nether_portal", 0),
        "lodestone" => ("minecraft:lodestone", 0),
        "test_instance_block" => ("minecraft:test_instance", 0),
        // PoiTypes.BEDS is the sixteen colours: no straw bed.
        bed if bed.ends_with("_bed") && bed != "straw_bed" => {
            if state.properties.get("part").map(String::as_str) != Some("head") {
                return None;
            }
            ("minecraft:home", 1)
        }
        rod if rod.ends_with("lightning_rod") => ("minecraft:lightning_rod", 0),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::BlockState;

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
