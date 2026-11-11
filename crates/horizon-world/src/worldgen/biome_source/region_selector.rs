//! Selector version 1 of a multi-noise preset's weighted mod regions (jittered cells), and the
//! biome source that resolves each quart in its owning region's own climate index.
use super::{BiomeSourceError, regions::RegionLayout};
use crate::worldgen::{
    biome::{BiomeError, BiomeResolver, MultiNoiseBiomeSource},
    climate::{ClimateSampler, ClimateVolume},
};
use horizon_core::{
    density::SamplerContext,
    position::BlockPos,
    random::{PositionalRandom, RandomSource},
};

/// The widest cell: squared distances across a 3x3 cell neighbourhood stay inside `i64`.
const MAX_CELL: u32 = 1 << 30;

/// Why a saved layout's selector cannot run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionError {
    /// A selector version this build does not know.
    Version(u32),
    /// A cell width outside `1..=2^30` quarts.
    Cell(u32),
    /// The weights sum to 0 or past `i32::MAX`.
    Weight,
}
impl std::fmt::Display for RegionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Version(version) => write!(f, "unknown region selector version {version}"),
            Self::Cell(cell) => write!(f, "region cell width {cell} is outside 1..={MAX_CELL}"),
            Self::Weight => f.write_str("region weights must sum to 1..=2147483647"),
        }
    }
}
impl std::error::Error for RegionError {}

type CellDraw = dyn Fn(i32, i32) -> [i32; 3] + Send + Sync;

/// Which slot owns each column: slot 0 is vanilla, slot `i + 1` the layout's region `i`.
pub struct RegionSelector {
    cell: i32,
    /// Each slot's exclusive end on the cumulative weight line.
    ends: Vec<i32>,
    /// A cell's jitter x, jitter z and weight pick, drawn in that order.
    draw: Box<CellDraw>,
}
impl std::fmt::Debug for RegionSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegionSelector")
            .field("cell", &self.cell)
            .field("ends", &self.ends)
            .finish_non_exhaustive()
    }
}
impl RegionSelector {
    /// Reads `root` (the generator's positional random) without advancing it.
    pub fn new<P>(layout: &RegionLayout, root: &P) -> Result<Self, RegionError>
    where
        P: PositionalRandom,
        <P::Source as RandomSource>::Positional: Send + Sync + 'static,
    {
        let params = &layout.selector;
        if params.version != 1 {
            return Err(RegionError::Version(params.version));
        }
        if !(1..=MAX_CELL).contains(&params.cell) {
            return Err(RegionError::Cell(params.cell));
        }
        let cell = params.cell as i32;
        let mut total = 0i32;
        let mut ends = Vec::with_capacity(layout.regions.len() + 1);
        for weight in
            std::iter::once(params.vanilla_weight).chain(layout.regions.iter().map(|r| r.weight))
        {
            total = i32::try_from(weight)
                .ok()
                .and_then(|weight| total.checked_add(weight))
                .ok_or(RegionError::Weight)?;
            ends.push(total);
        }
        if total == 0 {
            return Err(RegionError::Weight);
        }
        let stream = root.from_hash_of(&params.salt).fork_positional();
        let draw = move |x, z| {
            let mut random = stream.at(BlockPos::new(x, 0, z));
            let mut next = |bound| {
                random
                    .next_int_bound(bound)
                    .expect("cell and total weight are checked positive")
            };
            [next(cell), next(cell), next(total)]
        };
        Ok(Self {
            cell,
            ends,
            draw: Box::new(draw),
        })
    }
    /// The cell's jitter point in quarts and the slot its pick chose.
    fn seed(&self, cell_x: i64, cell_z: i64) -> (i64, i64, usize) {
        let [x, z, pick] = (self.draw)(cell_x as i32, cell_z as i32);
        let cell = i64::from(self.cell);
        (
            cell_x * cell + i64::from(x),
            cell_z * cell + i64::from(z),
            self.ends.partition_point(|&end| end <= pick),
        )
    }
    /// The slot a cell's pick chose (cell coordinates, not quarts).
    pub fn cell_slot(&self, cell_x: i32, cell_z: i32) -> usize {
        self.seed(cell_x.into(), cell_z.into()).2
    }
    /// The slot owning the column: the nearest jitter point among the 3x3 cells around it,
    /// the first one on a tie.
    pub fn owner(&self, quart_x: i32, quart_z: i32) -> usize {
        let (qx, qz) = (i64::from(quart_x), i64::from(quart_z));
        let cell = i64::from(self.cell);
        let (cx, cz) = (qx.div_euclid(cell), qz.div_euclid(cell));
        let mut best = (i64::MAX, 0);
        for dz in -1..=1 {
            for dx in -1..=1 {
                let (x, z, slot) = self.seed(cx + dx, cz + dz);
                let distance = (x - qx) * (x - qx) + (z - qz) * (z - qz);
                if distance < best.0 {
                    best = (distance, slot);
                }
            }
        }
        best.1
    }
}

/// A multi-noise preset with a world's regions: each column's owner picks the climate index
/// its quarts are searched in.
#[derive(Debug)]
pub struct RegionBiomeSource {
    /// Vanilla's index first, then each region's that is not substituted.
    indices: Vec<MultiNoiseBiomeSource>,
    /// Per slot, its index in `indices`; a substituted region routes to vanilla's.
    routes: Vec<usize>,
    selector: RegionSelector,
}
impl RegionBiomeSource {
    pub fn new<P>(
        vanilla: MultiNoiseBiomeSource,
        layout: &RegionLayout,
        root: &P,
    ) -> Result<Self, BiomeSourceError>
    where
        P: PositionalRandom,
        <P::Source as RandomSource>::Positional: Send + Sync + 'static,
    {
        let selector = RegionSelector::new(layout, root)?;
        let mut indices = vec![vanilla];
        let mut routes = vec![0];
        for region in &layout.regions {
            if region.substituted {
                routes.push(0);
                continue;
            }
            let entries = region
                .points
                .iter()
                .map(|point| Ok((point.parameter_point()?, point.biome.clone())))
                .collect::<Result<_, BiomeError>>()?;
            routes.push(indices.len());
            indices.push(MultiNoiseBiomeSource::new(entries)?);
        }
        Ok(Self {
            indices,
            routes,
            selector,
        })
    }
    pub fn selector(&self) -> &RegionSelector {
        &self.selector
    }
    /// Vanilla's biomes in its parameter order, then each resolving region's.
    pub fn biomes(&self) -> impl Iterator<Item = &str> {
        self.indices.iter().flat_map(|index| {
            index
                .parameters()
                .entries()
                .iter()
                .map(|(_, name)| name.as_str())
        })
    }
    /// One search cache per index; keep one resolver per worker.
    pub fn resolver<'a>(&'a self, climate: &'a ClimateSampler) -> RegionResolver<'a> {
        RegionResolver {
            source: self,
            searches: self
                .indices
                .iter()
                .map(|index| index.resolver(climate))
                .collect(),
            column: None,
        }
    }
}
#[derive(Debug)]
pub struct RegionResolver<'a> {
    source: &'a RegionBiomeSource,
    searches: Vec<BiomeResolver<'a>>,
    /// The last column asked and its owner's search: a column's quarts come together.
    column: Option<([i32; 2], usize)>,
}
impl<'a> RegionResolver<'a> {
    fn search(&mut self, quart: [i32; 3]) -> &mut BiomeResolver<'a> {
        let column = [quart[0], quart[2]];
        let search = match self.column {
            Some((last, search)) if last == column => search,
            _ => {
                let search = self.source.routes[self.source.selector.owner(column[0], column[1])];
                self.column = Some((column, search));
                search
            }
        };
        &mut self.searches[search]
    }
    pub fn sample(
        &mut self,
        context: &mut SamplerContext,
        quart: [i32; 3],
    ) -> Result<&'a str, BiomeError> {
        self.search(quart).sample(context, quart)
    }
    /// Every index reads the same climate, so one volume serves them all.
    pub fn prepare_volume(
        &self,
        context: &mut SamplerContext,
        minimum: [i32; 3],
        extents: [i32; 3],
    ) -> Result<ClimateVolume, BiomeError> {
        self.searches[0].prepare_volume(context, minimum, extents)
    }
    pub fn sample_prepared(
        &mut self,
        volume: &ClimateVolume,
        quart: [i32; 3],
    ) -> Result<&'a str, BiomeError> {
        self.search(quart).sample_prepared(volume, quart)
    }
}
