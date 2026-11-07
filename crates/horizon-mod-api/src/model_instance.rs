//! Instanced copies of one resolved standalone model, submitted for a single world view.
//!
//! `ModelInstance::tint` is `0xAARRGGBB` in vanilla's space, the same packed colour as
//! [`crate::WorldQuad::color`]. [`LightCoords`] is block and sky light, each `0..=15`, in
//! vanilla's own convention. The engine converts tint and light to the frame format.
//! Mods never read or write a packed light word, and never assume the frame format.

/// Block and sky light, each `0..=15`, in vanilla's convention.
///
/// [`LightCoords::new`] is the safe constructor. A wasm guest can write the fields
/// directly; the draw call then returns [`InstanceError::InvalidLight`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct LightCoords {
    /// Block light, `0..=15`.
    pub block: u8,
    /// Sky light, `0..=15`.
    pub sky: u8,
}

impl LightCoords {
    /// Both channels at 15.
    pub const FULL_BRIGHT: Self = Self {
        block: 15,
        sky: 15,
    };

    /// `block` and `sky` each in `0..=15`, or `None`.
    pub fn new(block: u8, sky: u8) -> Option<Self> {
        (block <= 15 && sky <= 15).then_some(Self { block, sky })
    }
}

/// Vanilla `OverlayTexture`: white-flash progress and the hurt tint.
///
/// Hurt hides the white flash, as `LivingEntityRenderer.getOverlayCoords` does.
/// [`Overlay::new`] is the safe constructor for the `0..=15` white column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Overlay {
    /// White flash column, `0..=15`. Zero is no white flash.
    pub white: u8,
    /// The hurt red. When set, the white flash is not drawn.
    pub hurt: bool,
}

impl Overlay {
    /// No white flash and no hurt tint.
    pub const NONE: Self = Self {
        white: 0,
        hurt: false,
    };

    /// `white` in `0..=15`, or `None`.
    pub fn new(white: u8, hurt: bool) -> Option<Self> {
        (white <= 15).then_some(Self { white, hurt })
    }
}

/// Row-major 3×4 identity, local to a copy's origin.
pub const IDENTITY_TRANSFORM: [f32; 12] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
];

/// One copy of a resolved model.
///
/// `origin` is the world-space position in blocks. The engine subtracts the camera in
/// `f64` before any `f32` cast. `transform` is row-major 3×4 and applies in that local
/// space, before the origin. Tint and light stay in vanilla's space.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ModelInstance {
    /// World-space origin, in blocks.
    pub origin: [f64; 3],
    /// Row-major 3×4 affine transform, local to `origin`.
    pub transform: [f32; 12],
    /// Packed `0xAARRGGBB` tint, the same convention as [`crate::WorldQuad::color`].
    pub tint: u32,
    /// Block and sky light in vanilla's `0..=15` convention.
    pub light: LightCoords,
    /// Hurt tint and white flash, in vanilla's overlay convention.
    pub overlay: Overlay,
}

impl ModelInstance {
    /// One copy. The draw call checks finiteness, light range and the view budget.
    pub fn new(
        origin: [f64; 3],
        transform: [f32; 12],
        tint: u32,
        light: LightCoords,
        overlay: Overlay,
    ) -> Self {
        Self {
            origin,
            transform,
            tint,
            light,
            overlay,
        }
    }

    /// Identity transform at `origin`, full bright, with no overlay.
    pub fn at(origin: [f64; 3], tint: u32) -> Self {
        Self::new(
            origin,
            IDENTITY_TRANSFORM,
            tint,
            LightCoords::FULL_BRIGHT,
            Overlay::NONE,
        )
    }
}

/// Why a whole instance batch was refused. Nothing from that batch is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InstanceError {
    /// The handle is stale or not a resolved standalone model.
    Model(crate::ModelError),
    /// An origin or transform component is not finite.
    NonFinite,
    /// A light channel is outside `0..=15`, including a field written past [`LightCoords::new`].
    InvalidLight,
    /// The batch would exceed this view's 16,384 instance budget.
    CapacityExceeded,
    /// The model has a translucent face. It is not drawn as solid.
    Translucent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_overlay_constructors_reject_a_channel_past_15() {
        assert_eq!(LightCoords::new(15, 0).unwrap().block, 15);
        assert!(LightCoords::new(16, 0).is_none());
        assert!(Overlay::new(16, false).is_none());
        assert_eq!(LightCoords::FULL_BRIGHT.sky, 15);
        assert_eq!(ModelInstance::at([1.0, 2.0, 3.0], 0xFF80_C040).tint, 0xFF80_C040);
    }
}
