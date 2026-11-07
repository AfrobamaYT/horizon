//! Fluid states carried by the pinned vanilla block-state registry.
//! Flow updates, spreading and entity interaction are separate simulation work.
use crate::{
    chunk::BlockState,
    compression::invalid,
    registry::{BlockRegistry, BlockStateId},
};
use std::{io, sync::RwLock};

/// What a mod's world fluid does when it flows, as `FlowingFluid`'s overridable getters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModFluidRules {
    /// `Fluid.getTickDelay`.
    pub tick_delay: i32,
    /// `FlowingFluid.getDropOff`: the amount lost per block.
    pub drop_off: i32,
    /// `FlowingFluid.getSlopeFindDistance`.
    pub slope_find_distance: i32,
    /// `FlowingFluid.canConvertToSource`.
    pub converts_to_source: bool,
}

/// A world fluid a mod registered: its two registry names, the block that holds it in the
/// world and the bucket that carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModFluidType {
    /// `<namespace>:<path>`, which is also its block's id.
    pub id: String,
    /// `<namespace>:flowing_<path>`.
    pub flowing_id: String,
    /// The filled bucket, when the fluid has one.
    pub bucket: Option<String>,
    pub rules: ModFluidRules,
}

/// A registered [`ModFluidType`]: small and copied around as vanilla's kinds are. Saved and
/// cached by name, because its number is only good for this process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModFluid(u16);

/// Leaked so that names stay `&'static str`, as vanilla's are; there is one entry per fluid id.
static MOD_FLUIDS: RwLock<Vec<&'static ModFluidType>> = RwLock::new(Vec::new());

fn mod_fluid_table() -> std::sync::RwLockReadGuard<'static, Vec<&'static ModFluidType>> {
    MOD_FLUIDS.read().unwrap_or_else(|e| e.into_inner())
}

/// Registers a mod's fluid before any block table is built from the registry. Registering an id
/// again replaces what it does and keeps its number.
pub fn register_mod_fluid(fluid: ModFluidType) -> ModFluid {
    let mut table = MOD_FLUIDS.write().unwrap_or_else(|e| e.into_inner());
    let at = table.iter().position(|known| known.id == fluid.id);
    let leaked: &'static ModFluidType = Box::leak(Box::new(fluid));
    match at {
        Some(at) => {
            table[at] = leaked;
            ModFluid(at as u16)
        }
        None => {
            table.push(leaked);
            ModFluid(u16::try_from(table.len() - 1).expect("fewer than 65536 mod fluids"))
        }
    }
}

/// Every registered mod fluid, in registration order.
pub fn mod_fluids() -> Vec<(ModFluid, &'static ModFluidType)> {
    mod_fluid_table()
        .iter()
        .enumerate()
        .map(|(at, fluid)| (ModFluid(at as u16), *fluid))
        .collect()
}

/// The mod fluid whose block or source fluid is `id`.
pub fn mod_fluid_named(id: &str) -> Option<ModFluid> {
    mod_fluid_table()
        .iter()
        .position(|fluid| fluid.id == id)
        .map(|at| ModFluid(at as u16))
}

/// The mod fluid a filled bucket holds.
pub fn mod_fluid_of_bucket(bucket: &str) -> Option<ModFluid> {
    mod_fluid_table()
        .iter()
        .position(|fluid| fluid.bucket.as_deref() == Some(bucket))
        .map(|at| ModFluid(at as u16))
}

impl ModFluid {
    /// Its place in [`mod_fluids`], for a table built from that list.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
    /// What was registered for this fluid.
    pub fn get(self) -> &'static ModFluidType {
        mod_fluid_table()[usize::from(self.0)]
    }
}
impl serde::Serialize for ModFluid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.get().id)
    }
}
impl<'de> serde::Deserialize<'de> for ModFluid {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        mod_fluid_named(&id)
            .ok_or_else(|| serde::de::Error::custom(format!("unregistered mod fluid {id}")))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FluidKind {
    Empty,
    FlowingWater,
    Water,
    FlowingLava,
    Lava,
    /// A mod's fluid, as a source.
    Mod(ModFluid),
    /// A mod's fluid, flowing.
    ModFlowing(ModFluid),
}
impl FluidKind {
    /// Vanilla's registry, in its order. A mod's fluids come after it (`mod_fluids`).
    pub const ALL: [Self; 5] = [
        Self::Empty,
        Self::FlowingWater,
        Self::Water,
        Self::FlowingLava,
        Self::Lava,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Empty => "minecraft:empty",
            Self::FlowingWater => "minecraft:flowing_water",
            Self::Water => "minecraft:water",
            Self::FlowingLava => "minecraft:flowing_lava",
            Self::Lava => "minecraft:lava",
            Self::Mod(fluid) => &fluid.get().id,
            Self::ModFlowing(fluid) => &fluid.get().flowing_id,
        }
    }
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name() == name)
            .or_else(|| {
                mod_fluid_table()
                    .iter()
                    .enumerate()
                    .find_map(|(at, fluid)| {
                        let id = ModFluid(at as u16);
                        (fluid.id == name)
                            .then_some(Self::Mod(id))
                            .or((fluid.flowing_id == name).then_some(Self::ModFlowing(id)))
                    })
            })
    }
    /// Its place in vanilla's fluid registry, the order of [`Self::ALL`]; a mod's fluid has none.
    pub const fn vanilla_index(self) -> Option<u8> {
        Some(match self {
            Self::Empty => 0,
            Self::FlowingWater => 1,
            Self::Water => 2,
            Self::FlowingLava => 3,
            Self::Lava => 4,
            Self::Mod(_) | Self::ModFlowing(_) => return None,
        })
    }
    /// The number a scheduled tick carries: vanilla's registry id, then the mods' fluids. Only
    /// good within a process; a save holds [`Self::name`].
    pub const fn tick_id(self) -> u32 {
        match self {
            Self::Mod(fluid) => 5 + 2 * fluid.0 as u32,
            Self::ModFlowing(fluid) => 6 + 2 * fluid.0 as u32,
            Self::Empty => 0,
            Self::FlowingWater => 1,
            Self::Water => 2,
            Self::FlowingLava => 3,
            Self::Lava => 4,
        }
    }
    /// The kind whose [`Self::tick_id`] is `id`.
    pub fn from_tick_id(id: u32) -> Option<Self> {
        if let Some(&kind) = Self::ALL.get(id as usize) {
            return Some(kind);
        }
        let at = (id - 5) / 2;
        let fluid = ModFluid(u16::try_from(at).ok()?);
        (usize::from(fluid.0) < mod_fluid_table().len()).then_some(if id % 2 == 1 {
            Self::Mod(fluid)
        } else {
            Self::ModFlowing(fluid)
        })
    }
    /// The mod fluid this is a form of.
    pub const fn mod_fluid(self) -> Option<ModFluid> {
        match self {
            Self::Mod(fluid) | Self::ModFlowing(fluid) => Some(fluid),
            _ => None,
        }
    }
    pub const fn same_family(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Empty, Self::Empty)
                | (
                    Self::Water | Self::FlowingWater,
                    Self::Water | Self::FlowingWater
                )
                | (
                    Self::Lava | Self::FlowingLava,
                    Self::Lava | Self::FlowingLava
                )
        ) || match (self.mod_fluid(), other.mod_fluid()) {
            (Some(a), Some(b)) => a.0 == b.0,
            _ => false,
        }
    }
    /// Its bit in a vanilla fluid tag's mask; a mod's fluid is in none of them.
    pub const fn mask(self) -> u8 {
        match self.vanilla_index() {
            Some(index) => 1 << index,
            None => 0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FluidState {
    kind: FluidKind,
    amount: u8,
    falling: bool,
}
impl FluidState {
    /// Native fluid states require an object; properties decode leniently.
    pub fn from_json(json: &str) -> io::Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| invalid(e.to_string()))?;
        let name = value
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid("missing fluid identifier"))?;
        let (namespace, path) = name.split_once(':').unwrap_or(("minecraft", name));
        let name = format!(
            "{}:{path}",
            if namespace.is_empty() {
                "minecraft"
            } else {
                namespace
            }
        );
        let kind =
            FluidKind::from_name(&name).ok_or_else(|| invalid("unknown fluid identifier"))?;
        if kind == FluidKind::Empty {
            return Ok(Self::EMPTY);
        }
        let flowing = matches!(
            kind,
            FluidKind::FlowingWater | FluidKind::FlowingLava | FluidKind::ModFlowing(_)
        );
        let mut state = Self {
            kind,
            amount: if flowing { 1 } else { 8 },
            falling: true,
        };
        if let Some(properties) = value
            .get("properties")
            .and_then(serde_json::Value::as_object)
        {
            if let Some(value) = properties
                .get("falling")
                .and_then(serde_json::Value::as_str)
            {
                match value {
                    "true" => state.falling = true,
                    "false" => state.falling = false,
                    _ => {}
                }
            }
            if flowing
                && let Some(value) = properties
                    .get("level")
                    .and_then(serde_json::Value::as_str)
                    .and_then(crate::registry::java_integer)
                    .filter(|v| (1..=8).contains(v))
            {
                state.amount = value as u8;
            }
        }
        Ok(state)
    }
    pub fn to_json(self) -> io::Result<String> {
        let mut value = serde_json::json!({"id": self.kind.name()});
        if self.kind != FluidKind::Empty {
            let mut properties = serde_json::json!({"falling": self.falling.to_string()});
            if !self.is_source() {
                properties["level"] = self.amount.to_string().into();
            }
            value["properties"] = properties;
        }
        serde_json::to_string(&value).map_err(|e| invalid(e.to_string()))
    }
    /// A fluid state from its parts; the empty fluid ignores amount and falling.
    pub const fn new(kind: FluidKind, amount: u8, falling: bool) -> Self {
        if matches!(kind, FluidKind::Empty) {
            return Self::EMPTY;
        }
        Self {
            kind,
            amount,
            falling,
        }
    }
    pub const EMPTY: Self = Self {
        kind: FluidKind::Empty,
        amount: 0,
        falling: false,
    };
    pub const WATER: Self = Self {
        kind: FluidKind::Water,
        amount: 8,
        falling: false,
    };
    pub const fn kind(self) -> FluidKind {
        self.kind
    }
    pub const fn amount(self) -> u8 {
        self.amount
    }
    pub const fn falling(self) -> bool {
        self.falling
    }
    pub const fn is_source(self) -> bool {
        matches!(
            self.kind,
            FluidKind::Water | FluidKind::Lava | FluidKind::Mod(_)
        )
    }
    pub const fn is_full(self) -> bool {
        self.amount == 8
    }
    pub fn height_at<W: FluidHeightWorld>(
        self,
        world: &mut W,
        position: [i32; 3],
    ) -> Result<f32, W::Error> {
        if self.kind == FluidKind::Empty {
            return Ok(0.0);
        }
        let above = [position[0], position[1].wrapping_add(1), position[2]];
        Ok(if self.kind.same_family(world.fluid(above)?.kind) {
            1.0
        } else {
            self.own_height()
        })
    }
    pub fn camera_height_at<W: FluidHeightWorld>(
        self,
        world: &mut W,
        position: [i32; 3],
    ) -> Result<f32, W::Error> {
        if self.is_source() {
            let above = [position[0], position[1].wrapping_add(1), position[2]];
            if world.sturdy_down(above)? {
                return Ok(1.0);
            }
        }
        self.height_at(world, position)
    }
    pub fn own_height(self) -> f32 {
        f32::from(self.amount) / 9.0
    }
    pub fn from_block(state: &BlockState) -> io::Result<Self> {
        let water = state.name == "minecraft:water";
        let mod_fluid = mod_fluid_named(&state.name);
        if water || state.name == "minecraft:lava" || mod_fluid.is_some() {
            let level = state
                .properties
                .get("level")
                .and_then(|v| v.parse::<u8>().ok())
                .filter(|v| *v <= 15)
                .ok_or_else(|| invalid("invalid liquid block level"))?;
            let kind = match (mod_fluid, water, level == 0) {
                (Some(fluid), _, true) => FluidKind::Mod(fluid),
                (Some(fluid), _, false) => FluidKind::ModFlowing(fluid),
                (None, true, true) => FluidKind::Water,
                (None, true, false) => FluidKind::FlowingWater,
                (None, false, true) => FluidKind::Lava,
                (None, false, false) => FluidKind::FlowingLava,
            };
            return Ok(Self {
                kind,
                amount: if level == 0 || level >= 8 {
                    8
                } else {
                    8 - level
                },
                falling: level >= 8,
            });
        }
        if matches!(
            state.name.as_str(),
            "minecraft:kelp"
                | "minecraft:kelp_plant"
                | "minecraft:seagrass"
                | "minecraft:tall_seagrass"
                | "minecraft:bubble_column"
        ) {
            return Ok(Self::WATER);
        }
        match state.properties.get("waterlogged").map(String::as_str) {
            Some("true") => {
                // The pinned copper-grate blocks use falling source water.
                let falling = matches!(
                    state.name.as_str(),
                    "minecraft:copper_grate"
                        | "minecraft:exposed_copper_grate"
                        | "minecraft:weathered_copper_grate"
                        | "minecraft:oxidized_copper_grate"
                        | "minecraft:waxed_copper_grate"
                        | "minecraft:waxed_exposed_copper_grate"
                        | "minecraft:waxed_weathered_copper_grate"
                        | "minecraft:waxed_oxidized_copper_grate"
                );
                Ok(Self {
                    falling,
                    ..Self::WATER
                })
            }
            None | Some("false") => Ok(Self::EMPTY),
            _ => Err(invalid("invalid waterlogged block property")),
        }
    }
}
/// `state`'s fluid when its block is a mod fluid's, whose rows copy a vanilla liquid's.
pub fn mod_fluid_of_block(state: &BlockState) -> io::Result<Option<FluidState>> {
    match mod_fluid_named(&state.name) {
        Some(_) => FluidState::from_block(state).map(Some),
        None => Ok(None),
    }
}
/// Queries required by fluid surface height and camera immersion. The caller
/// owns block shape/support behavior and world-query failures.
pub trait FluidHeightWorld {
    type Error;
    fn fluid(&mut self, position: [i32; 3]) -> Result<FluidState, Self::Error>;
    fn sturdy_down(&mut self, position: [i32; 3]) -> Result<bool, Self::Error>;
}
#[derive(Debug)]
pub struct FluidStateTable(Vec<FluidState>);
impl FluidStateTable {
    pub fn new(registry: &BlockRegistry) -> io::Result<Self> {
        (0..registry.len())
            .map(|i| FluidState::from_block(registry.state(BlockStateId(i as u32)).unwrap()))
            .collect::<io::Result<Vec<_>>>()
            .map(Self)
    }
    pub fn get(&self, state: BlockStateId) -> Option<FluidState> {
        self.0.get(state.0 as usize).copied()
    }
}

/// Bound identities for converting fluid states into their legacy liquid blocks.
pub struct FluidBlockBindings {
    air: BlockStateId,
    water: [BlockStateId; 16],
    lava: [BlockStateId; 16],
    /// Each mod fluid's block by level, by registration order; none when this registry lacks it.
    mods: Vec<Option<[BlockStateId; 16]>>,
}
impl FluidBlockBindings {
    pub fn new(registry: &BlockRegistry) -> io::Result<Self> {
        let levels = |name: &str| -> io::Result<[BlockStateId; 16]> {
            let default = registry
                .default_state(name)
                .ok_or_else(|| invalid("missing liquid block"))?;
            let mut state = registry.state(default).unwrap().clone();
            let mut result = [default; 16];
            for (level, id) in result.iter_mut().enumerate() {
                state.properties.insert("level".into(), level.to_string());
                *id = registry
                    .id(&state)
                    .ok_or_else(|| invalid("missing liquid block level"))?;
            }
            Ok(result)
        };
        Ok(Self {
            air: registry
                .default_state("minecraft:air")
                .ok_or_else(|| invalid("missing air block"))?,
            water: levels("minecraft:water")?,
            lava: levels("minecraft:lava")?,
            mods: mod_fluids()
                .into_iter()
                .map(|(_, fluid)| levels(&fluid.id).ok())
                .collect(),
        })
    }
    pub fn block(&self, state: FluidState) -> BlockStateId {
        if state.kind == FluidKind::Empty {
            return self.air;
        }
        let level = if state.is_source() {
            0
        } else {
            8 - state.amount.min(8) + if state.falling { 8 } else { 0 }
        } as usize;
        if let Some(fluid) = state.kind.mod_fluid() {
            return self
                .mods
                .get(usize::from(fluid.0))
                .copied()
                .flatten()
                .map_or(self.air, |levels| levels[level]);
        }
        if state.kind.same_family(FluidKind::Water) {
            self.water[level]
        } else {
            self.lava[level]
        }
    }
}

/// A fluid state as its parts: kind, amount and falling.
impl From<&FluidState> for (FluidKind, u8, bool) {
    fn from(state: &FluidState) -> Self {
        (state.kind, state.amount, state.falling)
    }
}

/// A fluid state from its parts (`FluidState::new`).
impl From<(FluidKind, u8, bool)> for FluidState {
    fn from((kind, amount, falling): (FluidKind, u8, bool)) -> Self {
        Self::new(kind, amount, falling)
    }
}

#[cfg(test)]
mod mod_fluid_tests {
    use super::*;

    fn oil(id: &str, flowing_id: &str) -> ModFluid {
        register_mod_fluid(ModFluidType {
            id: id.into(),
            flowing_id: flowing_id.into(),
            bucket: Some(format!("{id}_bucket")),
            rules: ModFluidRules {
                tick_delay: 20,
                drop_off: 2,
                slope_find_distance: 2,
                converts_to_source: false,
            },
        })
    }
    fn block(name: &str, level: u8) -> BlockState {
        BlockState {
            name: name.into(),
            properties: [("level".to_string(), level.to_string())].into(),
        }
    }

    #[test]
    fn a_mod_fluids_block_states_read_as_vanilla_liquids_do() {
        let fluid = oil("test:oil", "test:flowing_oil");
        let source = FluidState::from_block(&block("test:oil", 0)).unwrap();
        assert_eq!(source.kind(), FluidKind::Mod(fluid));
        assert!(source.is_source());
        let flowing = FluidState::from_block(&block("test:oil", 3)).unwrap();
        assert_eq!(flowing.kind(), FluidKind::ModFlowing(fluid));
        assert_eq!((flowing.amount(), flowing.falling()), (5, false));
        let falling = FluidState::from_block(&block("test:oil", 8)).unwrap();
        assert_eq!((falling.amount(), falling.falling()), (8, true));
        assert_eq!(
            FluidState::from_block(&block("test:other", 0)).unwrap(),
            FluidState::EMPTY
        );
    }

    #[test]
    fn a_mod_fluid_is_its_own_family_and_in_no_vanilla_tag() {
        let a = oil("test:a", "test:flowing_a");
        let b = oil("test:b", "test:flowing_b");
        assert!(FluidKind::Mod(a).same_family(FluidKind::ModFlowing(a)));
        assert!(!FluidKind::Mod(a).same_family(FluidKind::Mod(b)));
        assert!(!FluidKind::Mod(a).same_family(FluidKind::Water));
        assert!(!FluidKind::Water.same_family(FluidKind::ModFlowing(a)));
        assert_eq!(FluidKind::Mod(a).mask(), 0);
    }

    #[test]
    fn names_and_tick_ids_round_trip_and_vanilla_keeps_its_registry_ids() {
        for (id, kind) in FluidKind::ALL.into_iter().enumerate() {
            assert_eq!(kind.tick_id(), id as u32);
            assert_eq!(kind.vanilla_index(), Some(id as u8));
            assert_eq!(kind.mask(), 1 << id);
            assert_eq!(FluidKind::from_tick_id(id as u32), Some(kind));
        }
        let fluid = oil("test:round", "test:flowing_round");
        for kind in [FluidKind::Mod(fluid), FluidKind::ModFlowing(fluid)] {
            assert_eq!(FluidKind::from_name(kind.name()), Some(kind));
            assert_eq!(FluidKind::from_tick_id(kind.tick_id()), Some(kind));
            assert_eq!(kind.vanilla_index(), None);
        }
        assert_eq!(FluidKind::from_tick_id(u32::MAX), None);
        assert_eq!(FluidKind::from_name("test:unregistered"), None);
    }

    #[test]
    fn registering_an_id_again_keeps_its_number_and_a_saved_state_decodes_by_name() {
        let first = oil("test:again", "test:flowing_again");
        assert_eq!(oil("test:again", "test:flowing_again"), first);
        let state = FluidState::new(FluidKind::ModFlowing(first), 3, false);
        let json = state.to_json().unwrap();
        assert!(json.contains("test:flowing_again"));
        assert_eq!(FluidState::from_json(&json).unwrap(), state);
        let bytes = postcard::to_allocvec(&FluidKind::Mod(first)).unwrap();
        assert_eq!(
            postcard::from_bytes::<FluidKind>(&bytes).unwrap(),
            FluidKind::Mod(first)
        );
    }
}
