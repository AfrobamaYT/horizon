//! `neoforge:fluid_container` (`DynamicFluidContainerModel`): a container drawn
//! from one sprite per layer, its fluid layer the still sprite of the fluid the
//! stack holds through the `fluid` mask, and the fluid's tint on that layer.
use super::{Result, boolean, get, ident, object, required};
use crate::json::error as err;
use serde_json::{Map, Value};

/// What a `neoforge:fluid_container`'s fluid layer draws with: the still sprite
/// the fluid's state model uses (the block atlas's `block/water_still`) and the
/// colour `FluidContentsTint` gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FluidAppearance {
    pub still: String,
    pub tint: i32,
}

/// `DynamicFluidContainerModel.Textures`: every layer's sprite, each optional.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FluidContainerTextures {
    /// The container itself, drawn first.
    pub base: Option<String>,
    /// The mask the held fluid's still sprite is drawn through.
    pub fluid: Option<String>,
    /// The cover, always drawn as a mask: through it, `base` is drawn again over the
    /// fluid (`cover_is_mask`), or else the cover itself.
    pub cover: Option<String>,
}
impl FluidContainerTextures {
    /// The codec takes each sprite optionally and requires at least one texture.
    /// `particle` is checked for that count and for its shape but is not kept: an
    /// item's break particles come from each layer's own sprite.
    fn decode(m: &Map<String, Value>) -> Result<Self> {
        let textures = object(required(m, "textures")?)?;
        let sprite = |key: &str| get(textures, key).map(ident).transpose();
        let (base, fluid, cover) = (sprite("base")?, sprite("fluid")?, sprite("cover")?);
        let particle = sprite("particle")?;
        if particle.is_none() && base.is_none() && fluid.is_none() && cover.is_none() {
            return Err(err(
                "fluid container needs a particle, base, fluid or cover texture",
            ));
        }
        Ok(Self { base, fluid, cover })
    }
}

/// `DynamicFluidContainerModel.Unbaked`.
#[derive(Clone, Debug, PartialEq)]
pub struct FluidContainer {
    pub textures: FluidContainerTextures,
    /// The fluid an empty container draws; `minecraft:empty` draws no fluid layer.
    pub fluid: String,
    /// The cover masks `base` rather than drawing itself (`cover_is_mask`).
    pub cover_is_mask: bool,
}
impl FluidContainer {
    /// `flip_gas`, `apply_fluid_luminosity` and `force_opaque_fluid` are absent:
    /// the first needs a fluid type's density, which the host's fluid look does not
    /// carry, and the latter two need per-layer light and render types, which an
    /// item layer does not have yet.
    pub fn decode(m: &Map<String, Value>) -> Result<Self> {
        Ok(Self {
            textures: FluidContainerTextures::decode(m)?,
            fluid: ident(required(m, "fluid")?)?,
            cover_is_mask: get(m, "cover_is_mask").map_or(Ok(true), boolean)?,
        })
    }
}
