//! The section geometry design's acceptance cases (docs/modding/src/concepts/section-geometry.md)
//! that the mesher alone decides.
use super::mod_geometry::Renderer;
use super::mod_tint_tests::{resources, tint_values};
use super::*;
use horizon_assets::resource_source::ResourceRoot;
use mod_api::{BlockAndTintGetter, LightLayer, SectionGeometry, SectionQuad};
use std::sync::atomic::{AtomicUsize, Ordering};

fn quad(sprite: &str, translucent: bool) -> SectionQuad {
    SectionQuad {
        positions: [
            [2.0, 8.0, 2.0],
            [2.0, 8.0, 3.0],
            [3.0, 8.0, 3.0],
            [3.0, 8.0, 2.0],
        ],
        uvs: [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        sprite: sprite.into(),
        color: 0xFF20_4060,
        light: [15, 0],
        translucent,
    }
}

/// One decorative gold quad in every section, lit by the sky light the `level` reads there.
#[derive(Default)]
struct Decorative {
    calls: AtomicUsize,
}
impl SectionGeometry for Decorative {
    fn add(&self, origin: [i32; 3], level: &dyn BlockAndTintGetter) -> Vec<SectionQuad> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let mut quad = quad("minecraft:block/gold_block", false);
        let above = [origin[0] + 2, origin[1] + 8, origin[2] + 2];
        quad.light = [level.brightness(LightLayer::Sky, above) as u8, 3];
        vec![quad]
    }
}

/// A good quad beside a bad one, or too many quads: refused whole.
enum Bad {
    Corner(f32),
    Sprite,
    Many,
}
impl SectionGeometry for Bad {
    fn add(&self, _: [i32; 3], _: &dyn BlockAndTintGetter) -> Vec<SectionQuad> {
        let good = quad("minecraft:block/gold_block", false);
        let mut bad = good.clone();
        match self {
            Bad::Corner(value) => bad.positions[1][0] = *value,
            Bad::Sprite => bad.sprite = "missing:block/nothing".into(),
            Bad::Many => return vec![good; 262_145],
        }
        vec![good, bad]
    }
}

struct Panics;
impl SectionGeometry for Panics {
    fn add(&self, _: [i32; 3], _: &dyn BlockAndTintGetter) -> Vec<SectionQuad> {
        panic!("a broken renderer");
    }
}

struct Translucent;
impl SectionGeometry for Translucent {
    fn add(&self, _: [i32; 3], _: &dyn BlockAndTintGetter) -> Vec<SectionQuad> {
        vec![quad("minecraft:block/glass", true)]
    }
}

struct World {
    assets: Assets,
    render: RenderAssets,
    facts: BlockFacts,
    biomes: Biomes,
    root: std::path::PathBuf,
}

impl World {
    fn load() -> Option<Self> {
        let root = resources()?;
        let assets = Assets::load(&root).unwrap();
        let facts = BlockFacts::load(&root, assets.registry).unwrap();
        let biomes = Biomes::load(&ResourceRoot::directory(root.clone())).unwrap();
        let render = assets
            .reload(&ResourceRoot::directory(root.clone()))
            .unwrap();
        Some(Self {
            assets,
            render,
            facts,
            biomes,
            root,
        })
    }

    fn visibility(&self) -> FaceVisibility {
        crate::models::load_visibility(&self.root, self.assets.registry).unwrap()
    }

    /// Mesh tables whose section geometry is `renderers`.
    fn tables(&self, renderers: Vec<Arc<dyn SectionGeometry>>) -> MeshTables {
        let mut tables = MeshTables::new(
            &self.assets,
            &self.render,
            &self.facts,
            self.visibility(),
            None,
        )
        .unwrap();
        tables.section_geometry = renderers.into_iter().map(Renderer::new).collect();
        tables.mod_tints.values = Some(tint_values(self.assets.registry));
        tables
    }

    fn mesher<'a>(&'a self, tables: &'a MeshTables) -> Mesher<'a> {
        Mesher {
            nether_shading: false,
            shader_pack_lighting: false,
            separate_ao: false,
            shader_pack: false,
            assets: &self.assets,
            render: &self.render,
            tables,
            biomes: &self.biomes,
            visibility: self.visibility(),
            zoom: BiomeZoom::new(0),
            blend_radius: 0,
            smooth_lighting: false,
            cutout_leaves: true,
            opaque_ice: false,
            translucent_layers: &[],
            leaf_culling: LeafCullingMode::Off,
            hidden_states: &[],
        }
    }

    /// A chunk with one stone block in section 0 and section 1 empty.
    fn cube(&self) -> Chunk {
        let stone = self
            .assets
            .registry
            .default_state("minecraft:stone")
            .unwrap();
        let mut chunk = Chunk::empty(0, 0);
        chunk.set(2, MIN_Y + 4, 2, stone.0);
        chunk
    }
}

fn quads(vertices: &[TerrainVertex]) -> usize {
    vertices.len() / 4
}

#[test]
fn a_decorative_quad_joins_its_section_once_per_compilation() {
    let Some(world) = World::load() else {
        return;
    };
    let decorative = Arc::new(Decorative::default());
    let tables = world.tables(vec![decorative.clone()]);
    let mut mesher = world.mesher(&tables);
    let chunk = world.cube();
    let mut region = [[None; 3]; 3];
    region[1][1] = Some(&chunk);
    // Isolated cube plus one quad: 7 quads, the callback asked once.
    let cube = mesher.mesh(&region, 0);
    assert_eq!(quads(&cube.solid), 7);
    assert_eq!(decorative.calls.load(Ordering::Relaxed), 1);
    let gold = world.render.sprite_layers["minecraft:block/gold_block"] as u16;
    let added: Vec<_> = cube
        .solid
        .iter()
        .filter(|v| v.layer & !OPAQUE_LAYER == gold)
        .collect();
    assert_eq!(added.len(), 4);
    assert!(
        added.iter().all(|v| v.color == 0xFF60_4020),
        "ARGB packed as RGBA"
    );
    assert!(added.iter().all(|v| v.light == (15 * 16) | (3 * 16) << 8));
    // Empty section plus one quad: 1 quad after the build and after each of 3 remeshes into the
    // same buffers.
    let mut scratch = MeshScratch::default();
    let mut empty = mesher.mesh_reusing(&region, 1, SectionMesh::default(), &mut scratch);
    assert_eq!(quads(&empty.solid), 1);
    for _ in 0..3 {
        empty = mesher.mesh_reusing(&region, 1, empty, &mut scratch);
        assert_eq!(quads(&empty.solid), 1);
    }
    assert_eq!(decorative.calls.load(Ordering::Relaxed), 5);
}

#[test]
fn a_refused_contribution_adds_nothing_and_the_built_in_cube_keeps_six_faces() {
    let Some(world) = World::load() else {
        return;
    };
    let renderers: Vec<Arc<dyn SectionGeometry>> = vec![
        Arc::new(Panics),
        Arc::new(Bad::Corner(f32::NAN)),
        Arc::new(Bad::Corner(f32::INFINITY)),
        Arc::new(Bad::Corner(16.5)),
        Arc::new(Bad::Sprite),
        Arc::new(Bad::Many),
        Arc::new(Translucent),
    ];
    let tables = world.tables(renderers);
    let mut mesher = world.mesher(&tables);
    let chunk = world.cube();
    let mut region = [[None; 3]; 3];
    region[1][1] = Some(&chunk);
    for _ in 0..2 {
        let mesh = mesher.mesh(&region, 0);
        assert_eq!(quads(&mesh.solid), 6, "no offending quad added");
        assert_eq!(
            quads(&mesh.translucent),
            1,
            "the next renderer still adds its quad"
        );
    }
    assert!(
        tables.section_geometry[0].failed(),
        "the panicking one is disabled"
    );
    // No listeners: the mesh is the built-in one.
    let none = world.tables(Vec::new());
    let baseline = world.mesher(&none).mesh(&region, 0);
    assert_eq!(quads(&baseline.solid), 6);
    assert!(baseline.translucent.is_empty());
    assert!(world.mesher(&none).mesh(&region, 1).solid.is_empty());
}
