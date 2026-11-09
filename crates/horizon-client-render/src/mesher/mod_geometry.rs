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

/// One mod renderer, and whether it has panicked (then it is asked no more) or named a sprite the
/// atlas lacks (logged once).
pub(super) struct Renderer {
    renderer: Arc<dyn SectionGeometry>,
    failed: AtomicBool,
    missing_sprite: AtomicBool,
}

impl Renderer {
    pub(super) fn new(renderer: Arc<dyn SectionGeometry>) -> Self {
        Self {
            renderer,
            failed: AtomicBool::new(false),
            missing_sprite: AtomicBool::new(false),
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

/// Asks every renderer for the quads of the section `tints` is set up for, and adds them to `out`
/// after the section's own blocks.
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
        for quad in &quads {
            emit(mesher, renderer, quad, out);
        }
    }
}

/// `quad` as four section vertices in its pass, or nothing for a sprite the atlas lacks or a
/// corner that is not a number.
fn emit(mesher: &Mesher<'_>, renderer: &Renderer, quad: &SectionQuad, out: &mut SectionMesh) {
    let Some(&layer) = mesher.render.sprite_layers.get(&quad.sprite) else {
        if !renderer.missing_sprite.swap(true, Ordering::Relaxed) {
            log::warn!(
                "Horizon: section geometry names sprite {}, which the block atlas lacks",
                quad.sprite
            );
        }
        return;
    };
    if quad.positions.iter().flatten().any(|c| !c.is_finite()) {
        return;
    }
    let [a, r, g, b] = quad.color.to_be_bytes().map(u32::from);
    let color = r | g << 8 | b << 16 | a << 24;
    let [sky, block] = quad.light.map(|l| u16::from(l.min(15)) * 16);
    let light = sky | block << 8;
    let vertices = std::array::from_fn(|i| {
        TerrainVertex::new(quad.positions[i], quad.uvs[i], 0, color, light)
    });
    let tile = TileLayer::Layer {
        layer: layer as u16,
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
