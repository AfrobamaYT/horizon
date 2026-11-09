//! Section geometry native mods register (`add_section_geometry`, NeoForge's
//! `AddSectionGeometryEvent`): each renderer is asked on the meshing workers every time a section
//! meshes, and its quads go into that section's own mesh, so they follow its rebuilds and unload
//! with it.
use super::{Mesher, SectionMesh, TerrainVertex, Tints, mod_tint::Region};
use crate::connected::TileLayer;
use mod_api::{SectionGeometry, SectionQuad};
use std::{
    cell::RefCell,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

/// One mod renderer, and whether it has panicked (then it is asked no more) or had a list refused
/// (logged once).
pub(super) struct Renderer {
    renderer: Arc<dyn SectionGeometry>,
    failed: AtomicBool,
    refused: AtomicBool,
}

impl Renderer {
    pub(super) fn new(renderer: Arc<dyn SectionGeometry>) -> Self {
        Self {
            renderer,
            failed: AtomicBool::new(false),
            refused: AtomicBool::new(false),
        }
    }
    /// Whether it panicked, so that it is asked no more.
    #[cfg(test)]
    pub(super) fn failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }
}

/// The renderers, in registration order: set once at start, read by every table build.
static REGISTERED: OnceLock<Arc<[Renderer]>> = OnceLock::new();

/// Sets the native mods' section geometry renderers, once, before the resources first load; each
/// later [`super::MeshTables::new`] hands them to the meshing workers. False when already set.
pub fn register_section_geometry(renderers: Vec<Arc<dyn SectionGeometry>>) -> bool {
    REGISTERED
        .set(renderers.into_iter().map(Renderer::new).collect())
        .is_ok()
}

/// Whether any mod renderer is registered, so the game prepares the block values its `level`
/// reads.
pub fn has_section_geometry() -> bool {
    REGISTERED.get().is_some_and(|r| !r.is_empty())
}

/// What [`register_section_geometry`] set; none before.
pub(super) fn registered() -> Arc<[Renderer]> {
    REGISTERED.get().cloned().unwrap_or_else(|| Arc::new([]))
}

/// The most quads one renderer may add to one section (the section-meshing design's per-section
/// ceiling); a longer list is refused whole.
const MAX_QUADS: usize = 262_144;
/// How far outside its section a quad's corner may reach, in blocks: the one-block halo the
/// section is meshed with.
const HALO: f32 = 1.0;

/// Asks every renderer for the quads of the section `tints` is set up for, and adds them to `out`
/// after the section's own blocks. A renderer's list is taken whole or refused whole.
pub(super) fn add(mesher: &Mesher<'_>, tints: &mut Tints<'_>, out: &mut SectionMesh) {
    let [cx, cz, base_y] = tints.origin;
    let origin = [cx * 16, base_y, cz * 16];
    let region = Region {
        tables: mesher.tables,
        tints: RefCell::new(tints),
        nether: mesher.nether_shading,
    };
    for renderer in mesher.tables.section_geometry.iter() {
        if renderer.failed.load(Ordering::Relaxed) {
            continue;
        }
        let asked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            renderer.renderer.add(origin, &region)
        }));
        let Ok(quads) = asked else {
            if !renderer.failed.swap(true, Ordering::Relaxed) {
                log::warn!("Horizon: a section geometry renderer panicked; it is asked no more");
            }
            continue;
        };
        match layers(mesher, &quads) {
            Ok(layers) => {
                for (quad, layer) in quads.iter().zip(layers) {
                    emit(mesher, quad, layer, out);
                }
            }
            Err(why) => {
                if !renderer.refused.swap(true, Ordering::Relaxed) {
                    log::warn!(
                        "Horizon: section geometry for the section at {origin:?} was refused: {why}"
                    );
                }
            }
        }
    }
}

/// Each quad's atlas layer, or why the whole list is refused: too many quads, a sprite the atlas
/// lacks, or a corner or UV that is not a number or is out of range.
fn layers(mesher: &Mesher<'_>, quads: &[SectionQuad]) -> Result<Vec<u16>, String> {
    if quads.len() > MAX_QUADS {
        return Err(format!("{} quads, more than {MAX_QUADS}", quads.len()));
    }
    let corner = |c: &f32| c.is_finite() && (-HALO..=16.0 + HALO).contains(c);
    let uv = |c: &f32| c.is_finite() && (0.0..=1.0).contains(c);
    quads
        .iter()
        .map(|quad| {
            if !quad.positions.iter().flatten().all(corner) {
                return Err(format!(
                    "a corner outside the section: {:?}",
                    quad.positions
                ));
            }
            if !quad.uvs.iter().flatten().all(uv) {
                return Err(format!("a UV outside 0 to 1: {:?}", quad.uvs));
            }
            match mesher.render.sprite_layers.get(&quad.sprite) {
                Some(&layer) => Ok(layer as u16),
                None => Err(format!(
                    "sprite {}, which the block atlas lacks",
                    quad.sprite
                )),
            }
        })
        .collect()
}

/// `quad` as four section vertices in its pass, drawn with atlas `layer`.
fn emit(mesher: &Mesher<'_>, quad: &SectionQuad, layer: u16, out: &mut SectionMesh) {
    let [a, r, g, b] = quad.color.to_be_bytes().map(u32::from);
    let color = r | g << 8 | b << 16 | a << 24;
    let [sky, block] = quad.light.map(|l| u16::from(l.min(15)) * 16);
    let light = sky | block << 8;
    let vertices = std::array::from_fn(|i| {
        TerrainVertex::new(quad.positions[i], quad.uvs[i], 0, color, light)
    });
    let tile = TileLayer::Layer {
        layer,
        translucent: quad.translucent,
        opaque: false,
    };
    mesher.quad_as(
        out,
        false,
        tile,
        vertices,
        false,
        Some(quad.color & 0xFF_FFFF),
    );
}
