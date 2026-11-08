//! Ordered column obstruction checks, layer truncation and provider writes.
use super::{
    DecodeError, Object,
    block_feature::FeatureBlockBindings,
    block_predicate::BlockPredicate,
    feature::FeatureWorld,
    placement::{PlacementContext, PlacementQueryError},
    required,
    state_provider::{BlockStateProvider, StateProviderError},
    value_provider::IntProvider,
};
use horizon_core::{
    geometry::Direction,
    random::{InvalidBound, RandomSource},
};
use serde_json::value::RawValue;

#[derive(Clone, Debug)]
struct Layer {
    height: IntProvider,
    provider: BlockStateProvider,
}
impl Layer {
    fn decode_list(
        json: &str,
        providers: &super::state_provider::StateProviderBindings<'_>,
        catalog: &super::state_provider::StateProviderCatalog,
        depth: usize,
        remaining: &mut usize,
    ) -> Result<Vec<Self>, DecodeError> {
        let values: Vec<Box<RawValue>> = serde_json::from_str(json)?;
        let mut layers = Vec::with_capacity(values.len());
        for raw in values {
            let layer: Object = serde_json::from_str(raw.get())?;
            let height = IntProvider::decode(required(&layer, "height")?, depth + 1, remaining)?;
            if height.bounds()[0] < 0 {
                return Err(DecodeError::Field("negative block column height"));
            }
            let provider = catalog.provider_with_budget(
                required(&layer, "provider")?.get(),
                providers,
                depth + 1,
                remaining,
            )?;
            layers.push(Layer { height, provider });
        }
        Ok(layers)
    }
}
#[derive(Clone, Debug)]
pub struct BlockColumnFeature {
    layers: Vec<Layer>,
    direction: Direction,
    allowed: BlockPredicate,
    prioritize_tip: bool,
}
#[derive(Debug)]
pub enum BlockColumnError<E> {
    Random(InvalidBound),
    Provider(StateProviderError<E>),
    Query(PlacementQueryError),
    World(E),
    WorkLimit,
}
impl<E: std::fmt::Display> std::fmt::Display for BlockColumnError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Random(e) => e.fmt(f),
            Self::Provider(e) => e.fmt(f),
            Self::Query(e) => e.fmt(f),
            Self::World(e) => e.fmt(f),
            Self::WorkLimit => f.write_str("block column work limit exceeded"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for BlockColumnError<E> {}
impl BlockColumnFeature {
    pub fn from_json(json: &str, bindings: &FeatureBlockBindings<'_>) -> Result<Self, DecodeError> {
        Self::decode(json, bindings, 0, &mut 65536)
    }
    pub(crate) fn decode(
        json: &str,
        bindings: &FeatureBlockBindings<'_>,
        depth: usize,
        remaining: &mut usize,
    ) -> Result<Self, DecodeError> {
        if depth > 128 || *remaining == 0 {
            return Err(DecodeError::Field("block column depth or work"));
        }
        *remaining -= 1;
        let object: Object = serde_json::from_str(json)?;
        let layers = if let Some((providers, catalog, _)) = &bindings.simple_blocks {
            Layer::decode_list(
                required(&object, "layers")?.get(),
                providers,
                catalog,
                depth,
                remaining,
            )?
        } else {
            let values: Vec<Box<RawValue>> =
                serde_json::from_str(required(&object, "layers")?.get())?;
            if !values.is_empty() {
                return Err(DecodeError::Field("block column provider bindings"));
            }
            Vec::new()
        };
        let name: String = serde_json::from_str(required(&object, "direction")?.get())?;
        let direction = match name.as_str() {
            "down" => Direction::Down,
            "up" => Direction::Up,
            "north" => Direction::North,
            "south" => Direction::South,
            "west" => Direction::West,
            "east" => Direction::East,
            _ => return Err(DecodeError::Field("block column direction")),
        };
        let allowed = BlockPredicate::decode_with_blocks(
            required(&object, "allowed_placement")?.get(),
            depth + 1,
            remaining,
            Some(bindings),
        )?;
        let prioritize_tip = serde_json::from_str(required(&object, "prioritize_tip")?.get())?;
        Ok(Self {
            layers,
            direction,
            allowed,
            prioritize_tip,
        })
    }
    pub fn place<W: FeatureWorld + PlacementContext>(
        &self,
        world: &mut W,
        random: &mut impl RandomSource,
        origin: [i32; 3],
    ) -> Result<bool, BlockColumnError<W::Error>> {
        let mut heights = Vec::with_capacity(self.layers.len());
        let mut total = 0_i32;
        for layer in &self.layers {
            let height = layer
                .height
                .sample(random)
                .map_err(BlockColumnError::Random)?;
            total = total.wrapping_add(height);
            heights.push(height);
        }
        if total == 0 {
            return Ok(false);
        }
        let step = self.direction.normal();
        let advance = |p: [i32; 3]| std::array::from_fn(|i| p[i].wrapping_add(step[i]));
        let mut query_position = advance(origin);
        let mut budget = 1_048_576_usize;
        for index in 0..total {
            spend(&mut budget)?;
            if !self
                .allowed
                .test(world, query_position)
                .map_err(BlockColumnError::Query)?
            {
                truncate(&mut heights, total, index, self.prioritize_tip);
                break;
            }
            query_position = advance(query_position);
        }
        let mut position = origin;
        for (layer, height) in self.layers.iter().zip(heights) {
            for _ in 0..height {
                spend(&mut budget)?;
                let state = layer
                    .provider
                    .get_state(world, random, position)
                    .map_err(BlockColumnError::Provider)?;
                world
                    .set_block(position, state, 2)
                    .map_err(BlockColumnError::World)?;
                position = advance(position);
            }
        }
        Ok(true)
    }
}
fn spend<E>(budget: &mut usize) -> Result<(), BlockColumnError<E>> {
    if *budget == 0 {
        return Err(BlockColumnError::WorkLimit);
    }
    *budget -= 1;
    Ok(())
}
fn truncate(heights: &mut [i32], total: i32, allowed: i32, prioritize_tip: bool) {
    let mut remove = total.wrapping_sub(allowed);
    for offset in 0..heights.len() {
        if remove <= 0 {
            break;
        }
        let index = if prioritize_tip {
            offset
        } else {
            heights.len() - 1 - offset
        };
        let count = heights[index].min(remove);
        remove = remove.wrapping_sub(count);
        heights[index] = heights[index].wrapping_sub(count);
    }
}

#[cfg(test)]
mod ceiling_tests {
    use super::*;
    use crate::registry::BlockRegistry;
    use horizon_core::nbt::Tag;
    use std::collections::{BTreeMap, BTreeSet};
    #[test]
    fn column_layer_ceiling() {
        let registry = BlockRegistry::from_nbt(&Tag::compound([
            ("DataVersion", Tag::Int(crate::DATA_VERSION)),
            (
                "states",
                Tag::List(vec![Tag::compound([(
                    "id",
                    Tag::String("minecraft:stone".into()),
                )])]),
            ),
            (
                "defaults",
                Tag::compound([("minecraft:stone", Tag::Int(0))]),
            ),
        ]))
        .unwrap();
        let providers = super::super::state_provider::StateProviderBindings::new(
            &registry,
            BTreeSet::new(),
            &BTreeMap::new(),
        )
        .unwrap();
        let catalog =
            super::super::state_provider::StateProviderCatalog::from_resources(&BTreeMap::new())
                .unwrap();
        let layer = r#"{"height":0,"provider":{"id":"minecraft:stone"}}"#;
        let json = format!("[{}]", vec![layer; 8193].join(","));
        let layers = Layer::decode_list(&json, &providers, &catalog, 0, &mut 65536).unwrap();
        assert_eq!(layers.len(), 8193);
        assert_eq!(layers.last().unwrap().height.bounds(), [0, 0]);
    }
}
