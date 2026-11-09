use super::mod_geometry::Renderer;
use super::mod_tint_tests::{resources, tint_values};
use super::*;
use horizon_assets::resource_source::ResourceRoot;
use mod_api::{BlockAndTintGetter, LightLayer, SectionGeometry, SectionQuad};
use std::sync::Mutex;

/// Puts one gold quad on top of every stone block at section-local (2, 4, 2), another in the
/// translucent pass in every section, and one naming a sprite the atlas lacks; records each
/// section it is asked for.
struct Markers {
    asked: Mutex<Vec<[i32; 3]>>,
}

fn quad(y: f32, sprite: &str, translucent: bool, light: [u8; 2]) -> SectionQuad {
    SectionQuad {
        positions: [[2.0, y, 2.0], [2.0, y, 3.0], [3.0, y, 3.0], [3.0, y, 2.0]],
        uvs: [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        sprite: sprite.into(),
        color: 0xFF20_4060,
        light,
        translucent,
    }
}

impl SectionGeometry for Markers {
    fn add(&self, origin: [i32; 3], level: &dyn BlockAndTintGetter) -> Vec<SectionQuad> {
        self.asked.lock().unwrap().push(origin);
        let below = [origin[0] + 2, origin[1] + 4, origin[2] + 2];
        let mut quads = vec![
            quad(8.0, "minecraft:block/glass", true, [15, 0]),
            quad(9.0, "missing:block/nothing", false, [15, 0]),
        ];
        if level.block_state(below) != level.block_state([origin[0], origin[1] + 15, origin[2]]) {
            let sky = level.brightness(LightLayer::Sky, below) as u8;
            quads.push(quad(5.01, "minecraft:block/gold_block", false, [sky, 3]));
        }
        quads
    }
}

struct Panics;
impl SectionGeometry for Panics {
    fn add(&self, _: [i32; 3], _: &dyn BlockAndTintGetter) -> Vec<SectionQuad> {
        panic!("a broken renderer");
    }
}

/// The vertices of `mesh` drawn with atlas layer `layer`.
fn drawn_with(vertices: &[TerrainVertex], layer: u32) -> Vec<TerrainVertex> {
    vertices
        .iter()
        .filter(|v| u32::from(v.layer & !OPAQUE_LAYER) == layer)
        .copied()
        .collect()
}

#[test]
fn mod_quads_join_the_section_mesh_once_per_mesh_and_in_their_pass() {
    let Some(root) = resources() else {
        return;
    };
    let assets = Assets::load(&root).unwrap();
    let facts = BlockFacts::load(&root, assets.registry).unwrap();
    let biomes = Biomes::load(&ResourceRoot::directory(root.clone())).unwrap();
    let render = assets
        .reload(&ResourceRoot::directory(root.clone()))
        .unwrap();
    let visibility = crate::models::load_visibility(&root, assets.registry).unwrap();
    let mut tables = MeshTables::new(&assets, &render, &facts, visibility.clone(), None).unwrap();
    let markers = Arc::new(Markers {
        asked: Mutex::new(Vec::new()),
    });
    tables.section_geometry = Arc::new([
        Renderer::new(Arc::new(Panics)),
        Renderer::new(markers.clone()),
    ]);
    tables.mod_tints.values = Some(tint_values(assets.registry));
    let stone = assets.registry.default_state("minecraft:stone").unwrap();
    let mut chunk = Chunk::empty(0, 0);
    chunk.set(2, MIN_Y + 4, 2, stone.0);
    let mut region = [[None; 3]; 3];
    region[1][1] = Some(&chunk);
    let mut mesher = Mesher {
        nether_shading: false,
        shader_pack_lighting: false,
        separate_ao: false,
        shader_pack: false,
        assets: &assets,
        render: &render,
        tables: &tables,
        biomes: &biomes,
        visibility,
        zoom: BiomeZoom::new(0),
        blend_radius: 0,
        smooth_lighting: false,
        cutout_leaves: true,
        opaque_ice: false,
        translucent_layers: &[],
        leaf_culling: LeafCullingMode::Off,
        hidden_states: &[],
    };
    let gold = render.sprite_layers["minecraft:block/gold_block"];
    let glass = render.sprite_layers["minecraft:block/glass"];
    let mut scratch = MeshScratch::default();
    let first = mesher.mesh_reusing(&region, 0, SectionMesh::default(), &mut scratch);
    let solid = drawn_with(&first.solid, gold);
    assert_eq!(solid.len(), 4, "one gold quad over the stone");
    assert!(solid.iter().all(|v| v.pos[1] == 5.01));
    assert!(
        solid.iter().all(|v| v.color == 0xFF60_4020),
        "ARGB packed as RGBA"
    );
    assert!(solid.iter().all(|v| v.light == (15 * 16) | (3 * 16) << 8));
    assert_eq!(drawn_with(&first.translucent, glass).len(), 4);
    assert!(drawn_with(&first.solid, glass).is_empty());
    // Meshing the section again, into the same buffers, replaces its quads rather than adding.
    let again = mesher.mesh_reusing(&region, 0, first, &mut scratch);
    assert_eq!(drawn_with(&again.solid, gold).len(), 4);
    assert_eq!(drawn_with(&again.translucent, glass).len(), 4);
    // An all-air section is asked too, and the stone test there finds none.
    let air = mesher.mesh(&region, 1);
    assert!(drawn_with(&air.solid, gold).is_empty());
    assert_eq!(drawn_with(&air.translucent, glass).len(), 4);
    let origin = |sy: i32| [0, MIN_Y + sy * 16, 0];
    assert_eq!(
        *markers.asked.lock().unwrap(),
        [origin(0), origin(0), origin(1)],
        "the panicking renderer before it did not stop the rest"
    );
    assert!(tables.section_geometry[0].failed());
}
