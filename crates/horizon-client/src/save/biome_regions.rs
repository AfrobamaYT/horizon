//! A world's biome regions: `data/horizon/biome_regions.dat`, the region layout of each
//! dimension fixed when the world was created and changed only by its owner's recorded choice.
//! Design: `docs/modding/src/concepts/mod-biome-regions.md`.
use super::{DatIssue, Result, Tag, try_read_dat, write_dat_durable};
use horizon_core::nbt::JavaString;
use horizon_world::worldgen::biome_source::regions::{
    LayoutRegion, REGION_SELECTOR_VERSION, RegionLayout, RegionPoint, SelectorParams,
};
use mod_api::__private::BiomeRegionDeclaration;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(test)]
mod tests;

/// The record's format version.
const VERSION: u32 = 1;
/// What a substituted region resolves through: vanilla's region.
const VANILLA: &str = "minecraft:vanilla";
/// A point's climate intervals, in `RegionPoint::intervals` order.
const CLIMATE: [&str; 6] = [
    "temperature",
    "humidity",
    "continentalness",
    "erosion",
    "depth",
    "weirdness",
];

type Decoded<T> = std::result::Result<T, String>;

/// The record's file in `world`.
fn path(world: &Path) -> PathBuf {
    world.join("data").join("horizon").join("biome_regions.dat")
}

/// A world's saved region layouts by dimension id. A dimension it does not name keeps its
/// existing biome source.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BiomeRegions {
    dimensions: BTreeMap<String, Arc<RegionLayout>>,
}

/// How a world's record differs from the mods installed now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Differences {
    /// `(mod, region)` of each region whose mod is not installed and that has no recorded
    /// choice.
    missing: Vec<(String, String)>,
    /// Regions of installed mods that the record and the declarations do not hold alike:
    /// changed points or weight, or a region only one of them has.
    changed: Vec<String>,
    /// Substituted regions whose mod is installed again.
    returned: Vec<String>,
}

impl BiomeRegions {
    /// A new world's record: each declared region in its dimension at its declared weight,
    /// under the current selector. Only dimensions with a region are named.
    fn from_declarations(declarations: &[BiomeRegionDeclaration]) -> Self {
        let mut dimensions = BTreeMap::<String, RegionLayout>::new();
        for declaration in declarations {
            dimensions
                .entry(declaration.region.dimension.clone())
                .or_insert_with(|| RegionLayout {
                    selector: SelectorParams::current(),
                    regions: Vec::new(),
                })
                .regions
                .push(declared(declaration));
        }
        Self {
            dimensions: dimensions
                .into_iter()
                .map(|(id, layout)| (id, Arc::new(layout)))
                .collect(),
        }
    }

    /// The region layout the generator uses for `dimension` (`minecraft:overworld`), or `None`
    /// when the dimension keeps its existing source.
    pub fn layout(&self, dimension: &str) -> Option<Arc<RegionLayout>> {
        self.dimensions.get(dimension).cloned()
    }

    /// `world`'s record; `None` without the file. A file that does not read as a record is
    /// the reason why, and stays as it is.
    fn read(world: &Path) -> Decoded<Option<Self>> {
        match try_read_dat(&path(world)) {
            Ok(tag) => decode(&tag).map(Some),
            Err(DatIssue::Missing) => Ok(None),
            Err(DatIssue::Unreadable(why)) => Err(why),
        }
        .map_err(|why| format!("{}: {why}", path(world).display()))
    }

    /// Writes the record into `world`, synced to the disk before any chunk generates with it.
    fn write(&self, world: &Path) -> Result<()> {
        write_dat_durable(&path(world), self.encode())
    }

    fn differences(
        &self,
        installed: &BTreeSet<&str>,
        declarations: &[BiomeRegionDeclaration],
    ) -> Differences {
        let mut differences = Differences::default();
        for (dimension, layout) in &self.dimensions {
            for region in &layout.regions {
                match (
                    installed.contains(region.owner.as_str()),
                    region.substituted,
                ) {
                    (false, false) => differences
                        .missing
                        .push((region.owner.clone(), region.id.clone())),
                    (false, true) => {}
                    (true, true) => differences.returned.push(region.id.clone()),
                    (true, false) => {
                        if declarations
                            .iter()
                            .find(|d| d.region.dimension == *dimension && d.id == region.id)
                            .is_none_or(|d| declared(d) != *region)
                        {
                            differences.changed.push(region.id.clone());
                        }
                    }
                }
            }
        }
        for declaration in declarations {
            let saved = self
                .dimensions
                .get(&declaration.region.dimension)
                .is_some_and(|layout| layout.regions.iter().any(|r| r.id == declaration.id));
            if !saved {
                differences.changed.push(declaration.id.clone());
            }
        }
        differences
    }

    /// Records "vanilla's biomes" for each region whose mod is not installed: the region keeps
    /// its slot and weight, so the other regions' borders do not move.
    fn choose_vanilla(&mut self, installed: &BTreeSet<&str>) {
        for layout in self.dimensions.values_mut() {
            for region in &mut Arc::make_mut(layout).regions {
                if !installed.contains(region.owner.as_str()) {
                    region.substituted = true;
                }
            }
        }
    }

    /// The installed regions in place of the saved ones: a region the record already holds
    /// keeps its saved weight, a new one takes its declared weight, and a dimension the record
    /// names keeps its selector.
    fn adopted(&self, declarations: &[BiomeRegionDeclaration]) -> Self {
        let mut adopted = Self::from_declarations(declarations);
        for (dimension, layout) in &mut adopted.dimensions {
            let Some(saved) = self.dimensions.get(dimension) else {
                continue;
            };
            let layout = Arc::make_mut(layout);
            layout.selector = saved.selector.clone();
            for region in &mut layout.regions {
                if let Some(old) = saved.regions.iter().find(|r| r.id == region.id) {
                    region.weight = old.weight;
                }
            }
        }
        adopted
    }

    fn encode(&self) -> Tag {
        let dimensions = self
            .dimensions
            .iter()
            .map(|(id, layout)| (JavaString::from(id.as_str()), encode_layout(layout)))
            .collect();
        Tag::compound([
            ("version", Tag::Int(VERSION as i32)),
            ("dimensions", Tag::Compound(dimensions)),
        ])
    }
}

/// A declared region as a new world records it.
fn declared(declaration: &BiomeRegionDeclaration) -> LayoutRegion {
    LayoutRegion {
        owner: declaration.mod_id.clone(),
        id: declaration.id.clone(),
        weight: declaration.region.weight,
        points: declaration
            .region
            .points
            .iter()
            .map(|p| RegionPoint {
                biome: p.biome.clone(),
                intervals: [
                    p.temperature,
                    p.humidity,
                    p.continentalness,
                    p.erosion,
                    p.depth,
                    p.weirdness,
                ],
                offset: p.offset,
            })
            .collect(),
        substituted: false,
    }
}

/// The mods installed now: the loaded code mods and the mods whose regions are declared.
fn installed() -> BTreeSet<&'static str> {
    mod_api::__private::installed_code_ids()
        .chain(declarations().iter().map(|d| d.mod_id.as_str()))
        .collect()
}

fn declarations() -> &'static [BiomeRegionDeclaration] {
    mod_api::__private::biome_region_declarations()
}

/// A new world's record, written before its first chunk generates: the installed mods'
/// regions at their declared weights. Nothing is written when no mod declares one.
pub fn create(world: &Path) -> Result<()> {
    if declarations().is_empty() {
        return Ok(());
    }
    BiomeRegions::from_declarations(declarations()).write(world)
}

/// An existing world's record as it opens, with its owner's choices, for the generator
/// ([`BiomeRegions::layout`]); `None` without one, when every dimension keeps its source.
/// Refused with the reason while the record does not read (the file is left as it is) or a
/// missing mod's region has no recorded choice. A difference from the installed mods' regions
/// is reported, never taken.
pub fn open(world: &Path) -> Result<Option<BiomeRegions>> {
    Ok(open_with(world, &installed(), declarations())?)
}

fn open_with(
    world: &Path,
    installed: &BTreeSet<&str>,
    declarations: &[BiomeRegionDeclaration],
) -> Decoded<Option<BiomeRegions>> {
    let Some(record) = BiomeRegions::read(world)? else {
        return Ok(None);
    };
    let differences = record.differences(installed, declarations);
    if !differences.missing.is_empty() {
        let named: Vec<String> = differences
            .missing
            .iter()
            .map(|(owner, id)| format!("biome region {id} of mod {owner}"))
            .collect();
        return Err(format!(
            "the world's {} is missing; choose \"open with vanilla biomes\" in the world list, \
             or on a dedicated server set missing = \"vanilla\" under [biome_regions] in \
             server.toml",
            named.join(", ")
        ));
    }
    for id in &differences.changed {
        log::warn!(
            "Horizon: biome region {id} differs from the world's saved regions; the saved ones \
             stay until the world adopts the installed regions"
        );
    }
    for id in &differences.returned {
        log::warn!(
            "Horizon: the mod of biome region {id} is installed again; the world keeps vanilla's \
             biomes there until it adopts the installed regions"
        );
    }
    Ok(Some(record))
}

/// The region layout `world` saved for `dimension`, which its generator uses; `None` keeps the
/// dimension's existing source. [`open`] has already refused a record that does not read or
/// lacks a choice for a missing mod's region, and [`choose`] has written any choice made.
pub fn layout(world: &Path, dimension: &str) -> Result<Option<Arc<RegionLayout>>> {
    Ok(BiomeRegions::read(world)?.and_then(|record| record.layout(dimension)))
}

/// The regions of `world` whose mod is not installed and that have no recorded choice, as
/// `(mod, region)`. None when the record is absent, or does not read: opening refuses that
/// with the reason.
pub fn unchosen_missing(world: &Path) -> Vec<(String, String)> {
    BiomeRegions::read(world)
        .ok()
        .flatten()
        .map(|record| record.differences(&installed(), declarations()).missing)
        .unwrap_or_default()
}

/// Whether adopting the installed mods' regions would change `world`'s record, so the world's
/// owner has something to opt in to. False while the record does not read: opening refuses
/// that with the reason.
pub fn adoptable(world: &Path) -> bool {
    adoptable_with(world, declarations())
}

fn adoptable_with(world: &Path, declarations: &[BiomeRegionDeclaration]) -> bool {
    BiomeRegions::read(world).is_ok_and(|saved| {
        let saved = saved.unwrap_or_default();
        saved.adopted(declarations) != saved
    })
}

/// Records the owner's choices in `world`'s record before it opens: `adopt_installed` takes the
/// installed mods' regions, keeping the saved weights; `vanilla_for_missing` resolves the
/// regions of mods still missing through vanilla's. The record is written only when a
/// choice changed it; one that does not read is refused and left as it is.
pub fn choose(world: &Path, vanilla_for_missing: bool, adopt_installed: bool) -> Result<()> {
    choose_with(
        world,
        &installed(),
        declarations(),
        vanilla_for_missing,
        adopt_installed,
    )
}

fn choose_with(
    world: &Path,
    installed: &BTreeSet<&str>,
    declarations: &[BiomeRegionDeclaration],
    vanilla_for_missing: bool,
    adopt_installed: bool,
) -> Result<()> {
    let saved = BiomeRegions::read(world)?.unwrap_or_default();
    let mut chosen = if adopt_installed {
        saved.adopted(declarations)
    } else {
        saved.clone()
    };
    if vanilla_for_missing {
        chosen.choose_vanilla(installed);
    }
    if chosen != saved {
        chosen.write(world)?;
    }
    Ok(())
}

fn string(text: &str) -> Tag {
    Tag::String(text.into())
}

fn encode_layout(layout: &RegionLayout) -> Tag {
    let selector = &layout.selector;
    Tag::compound([
        (
            "selector",
            Tag::compound([
                ("version", Tag::Int(selector.version as i32)),
                ("cell", Tag::Int(selector.cell as i32)),
                ("salt", string(&selector.salt)),
                ("vanilla_weight", Tag::Int(selector.vanilla_weight as i32)),
            ]),
        ),
        (
            "regions",
            Tag::List(layout.regions.iter().map(encode_region).collect()),
        ),
    ])
}

fn encode_region(region: &LayoutRegion) -> Tag {
    let mut entries = vec![
        ("owner", string(&region.owner)),
        ("id", string(&region.id)),
        ("weight", Tag::Int(region.weight as i32)),
        (
            "points",
            Tag::List(region.points.iter().map(encode_point).collect()),
        ),
    ];
    if region.substituted {
        entries.push((
            "substituted",
            Tag::compound([
                ("with", string(VANILLA)),
                ("missing_mod", string(&region.owner)),
            ]),
        ));
    }
    Tag::compound(entries)
}

fn encode_point(point: &RegionPoint) -> Tag {
    let mut entries = vec![("biome", string(&point.biome))];
    entries.extend(
        CLIMATE
            .iter()
            .zip(&point.intervals)
            .map(|(&name, &(min, max))| {
                (
                    name,
                    Tag::List(vec![Tag::Float(min.to_bits()), Tag::Float(max.to_bits())]),
                )
            }),
    );
    entries.push(("offset", Tag::Float(point.offset.to_bits())));
    Tag::compound(entries)
}

fn field<'a>(tag: &'a Tag, key: &str, at: &str) -> Decoded<&'a Tag> {
    tag.get(key).ok_or_else(|| format!("{at}: no {key}"))
}

fn int(tag: &Tag, key: &str, at: &str) -> Decoded<u32> {
    match field(tag, key, at)? {
        Tag::Int(value) => u32::try_from(*value).map_err(|_| format!("{at}: {key} is negative")),
        _ => Err(format!("{at}: {key} is not an int")),
    }
}

fn text(tag: &Tag, key: &str, at: &str) -> Decoded<String> {
    match field(tag, key, at)? {
        Tag::String(value) => value
            .to_string()
            .map_err(|_| format!("{at}: {key} is not valid text")),
        _ => Err(format!("{at}: {key} is not a string")),
    }
}

fn float(tag: Option<&Tag>) -> Option<f32> {
    match tag {
        Some(&Tag::Float(bits)) => Some(f32::from_bits(bits)),
        _ => None,
    }
}

fn decode(tag: &Tag) -> Decoded<BiomeRegions> {
    let version = int(tag, "version", "record")?;
    if version != VERSION {
        return Err(format!("unknown record version {version}"));
    }
    let Some(Tag::Compound(dimensions)) = tag.get("dimensions") else {
        return Err("record: dimensions is not a compound".into());
    };
    let dimensions = dimensions
        .iter()
        .map(|(id, layout)| -> Decoded<(String, Arc<RegionLayout>)> {
            let id = id
                .to_string()
                .map_err(|_| "record: a dimension id is not valid text".to_owned())?;
            let layout = decode_layout(layout, &id)?;
            Ok((id, Arc::new(layout)))
        })
        .collect::<Decoded<_>>()?;
    Ok(BiomeRegions { dimensions })
}

fn decode_layout(tag: &Tag, dimension: &str) -> Decoded<RegionLayout> {
    let at = format!("{dimension} selector");
    let selector = field(tag, "selector", dimension)?;
    let selector = SelectorParams {
        version: int(selector, "version", &at)?,
        cell: int(selector, "cell", &at)?,
        salt: text(selector, "salt", &at)?,
        vanilla_weight: int(selector, "vanilla_weight", &at)?,
    };
    if selector.version != REGION_SELECTOR_VERSION {
        return Err(format!("{at}: unknown version {}", selector.version));
    }
    if selector.cell == 0 || selector.vanilla_weight == 0 {
        return Err(format!("{at}: cell and vanilla_weight must be above 0"));
    }
    let Some(Tag::List(regions)) = tag.get("regions") else {
        return Err(format!("{dimension}: regions is not a list"));
    };
    let regions = regions
        .iter()
        .map(|region| decode_region(region, dimension))
        .collect::<Decoded<Vec<_>>>()?;
    if !regions
        .windows(2)
        .all(|pair| (&pair[0].owner, &pair[0].id) < (&pair[1].owner, &pair[1].id))
    {
        return Err(format!(
            "{dimension}: regions are not in mod, then id, order, or one repeats"
        ));
    }
    let ids: BTreeSet<&str> = regions.iter().map(|r| r.id.as_str()).collect();
    if ids.len() != regions.len() {
        return Err(format!("{dimension}: a region id repeats"));
    }
    if regions
        .iter()
        .try_fold(selector.vanilla_weight, |total, r| {
            total.checked_add(r.weight)
        })
        .is_none()
    {
        return Err(format!("{dimension}: the weights' total overflows"));
    }
    Ok(RegionLayout { selector, regions })
}

fn decode_region(tag: &Tag, dimension: &str) -> Decoded<LayoutRegion> {
    let owner = text(tag, "owner", dimension)?;
    let id = text(tag, "id", dimension)?;
    let at = format!("{dimension} region {id}");
    let weight = int(tag, "weight", &at)?;
    let Some(Tag::List(points)) = tag.get("points") else {
        return Err(format!("{at}: points is not a list"));
    };
    let points = points
        .iter()
        .map(|point| decode_point(point, &at))
        .collect::<Decoded<Vec<_>>>()?;
    let substituted = match tag.get("substituted") {
        None => false,
        Some(choice) => {
            if text(choice, "with", &at)? != VANILLA || text(choice, "missing_mod", &at)? != owner {
                return Err(format!(
                    "{at}: substituted is not {VANILLA} for its own mod {owner}"
                ));
            }
            true
        }
    };
    // The same checks as a declaration's: a record holds only regions a mod could declare.
    mod_api::BiomeRegion {
        dimension: dimension.to_owned(),
        weight,
        points: points
            .iter()
            .map(|p| mod_api::BiomePoint {
                biome: p.biome.clone(),
                temperature: p.intervals[0],
                humidity: p.intervals[1],
                continentalness: p.intervals[2],
                erosion: p.intervals[3],
                depth: p.intervals[4],
                weirdness: p.intervals[5],
                offset: p.offset,
            })
            .collect(),
    }
    .check()
    .map_err(|error| format!("{at}: {error:?}"))?;
    Ok(LayoutRegion {
        owner,
        id,
        weight,
        points,
        substituted,
    })
}

fn decode_point(tag: &Tag, at: &str) -> Decoded<RegionPoint> {
    let biome = text(tag, "biome", at)?;
    let mut intervals = [(0.0, 0.0); 6];
    for (interval, name) in intervals.iter_mut().zip(CLIMATE) {
        *interval = match tag.get(name) {
            Some(Tag::List(pair)) if pair.len() == 2 => float(pair.first())
                .zip(float(pair.get(1)))
                .ok_or_else(|| format!("{at} point {biome}: {name} is not two floats"))?,
            _ => return Err(format!("{at} point {biome}: {name} is not [min, max]")),
        };
    }
    let offset =
        float(tag.get("offset")).ok_or_else(|| format!("{at} point {biome}: no float offset"))?;
    Ok(RegionPoint {
        biome,
        intervals,
        offset,
    })
}
