//! The typed submission surface; renderer storage is never exposed to mods.
use super::Game;
use super::mods;
use glam::{DVec3, Mat4, Vec3};
use horizon_client::mesher::Vertex;
use horizon_client::world_geometry::{self, ModelBatch, WorldGeometry};
use mod_api::{
    DrawError, InstanceError, ModelInstance, ModelRef, WorldDrawUsage, WorldLine, WorldQuad,
};

impl Game<'_> {
    /// `DebugRenderer.refreshRendererList`: the active renderers are replaced by those the mods
    /// push now whose entries are on. Runs at start, at login and when the selection changes.
    pub(in crate::game) fn refresh_debug_renderers(&mut self) {
        let renderers = mods::debug_renderers(self.options.execution_role);
        let reduced = self.reduced_debug_info();
        self.debug_renderers = self
            .debug_entries
            .active_renderers(renderers, self.debug, reduced);
    }
    /// `DebugRenderer.emitGizmos`'s first half: a selection the list has not seen refreshes it.
    fn follow_debug_selection(&mut self) {
        let version = self.debug_entries.enabled_version();
        if self.debug_renderers_version != Some(version) {
            self.debug_renderers_version = Some(version);
            self.refresh_debug_renderers();
        }
    }
    /// One view's geometry, the last view's gone first. `title` views draw no world: the
    /// title and connect screens have no mod geometry, so none outlives leaving a world.
    pub(super) fn fill_world_geometry(
        &mut self,
        title: bool,
        camera: [f64; 3],
        view: Mat4,
        alpha: f64,
        draw: &mut WorldGeometry,
        entities: &mut Vec<Vertex>,
    ) {
        let white = self.gui.atlas.sprites["horizon/white"];
        draw.reset(
            camera,
            view,
            [(white.u0 + white.u1) * 0.5, (white.v0 + white.v1) * 0.5],
        );
        if title {
            return;
        }
        let view = mod_api::WorldRenderView::new(camera, alpha as f32, self.world.game_time);
        self.native_submit_custom_geometry(&view, draw);
        self.follow_debug_selection();
        for id in &self.debug_renderers {
            self.native_debug_renderer_draw(id, &view, draw);
        }
        self.commit_model_instances(draw, DVec3::from(camera), entities);
    }
    /// Submit unlit ARGB world lines atomically. Invalid coordinates or widths,
    /// or a batch exceeding the view's fixed capacity, leave the batch unchanged.
    pub(super) fn native_world_draw_lines(
        &self,
        draw: &mut WorldGeometry,
        lines: &[WorldLine],
    ) -> Result<(), DrawError> {
        draw.lines(lines)
    }
    /// Submit unlit, alpha-blended world quads atomically, using the white atlas texel.
    pub(super) fn native_world_draw_quads(
        &self,
        draw: &mut WorldGeometry,
        quads: &[WorldQuad],
    ) -> Result<(), DrawError> {
        draw.quads(quads)
    }
    /// Queued counts and fixed capacities, shared by both depth modes and all listeners.
    /// Capacity is allocated during setup, never grown during submission.
    pub(super) fn native_world_draw_usage(&self, draw: &WorldGeometry) -> WorldDrawUsage {
        draw.usage()
    }
    /// Resolve `model` once and accept every copy, or leave the view unchanged.
    pub(super) fn native_world_draw_model_instances(
        &self,
        draw: &mut WorldGeometry,
        model: ModelRef,
        instances: &[ModelInstance],
    ) -> Result<(), InstanceError> {
        use mod_api::__private::game::Fields;
        let fields = model.fields();
        if fields.generation != self.standalone_geometry.generation() {
            return Err(InstanceError::Model(mod_api::ModelError::StaleModel));
        }
        let slot = self
            .standalone_geometry
            .instance_slot(fields.slot)
            .ok_or(InstanceError::Model(mod_api::ModelError::UnknownModel))?;
        let gpu = super::native_block_entity_draw::prepared_route(
            self.pack_ids.is_some(),
            !self.native_shaders.picked.is_empty(),
            self.renderer
                .as_ref()
                .and_then(|renderer| renderer.prepared_generation()),
            fields.generation,
        );
        draw.submit_model(
            ModelBatch {
                slot: fields.slot,
                generation: fields.generation,
                translucent: slot.translucent,
                bounds: slot.bounds,
                gpu,
            },
            instances,
            self.view_frustum.as_ref(),
        )
    }
    /// Stage the copies the instance buffer can take, and bake the rest onto the entity stream.
    fn commit_model_instances(
        &mut self,
        draw: &mut WorldGeometry,
        camera: DVec3,
        entities: &mut Vec<Vertex>,
    ) {
        if draw.copy_count() == 0 {
            return;
        }
        let wanted = (0..draw.copy_count())
            .filter(|&index| draw.copy(index).gpu && draw.copy(index).overlay == 0)
            .count();
        let already = self.prepared_model_draws.borrow().len() as u64;
        let mut gpu_left = self.prepared_copy_room(already, wanted).unwrap_or(0);
        let cut = entities.len() as u32;
        let mut staged = self.prepared_model_draws.borrow_mut();
        for index in 0..draw.copy_count() {
            let copy = draw.copy(index);
            let on_gpu = copy.gpu && copy.overlay == 0 && gpu_left > 0;
            if on_gpu {
                gpu_left -= 1;
                let mut instance = super::native_block_entity_draw::prepared_instance(
                    world_geometry::row_major_3x4(copy.transform),
                    DVec3::from(copy.origin),
                    camera,
                    copy.tint,
                    copy.light,
                );
                instance.tint[3] = (copy.tint >> 24) as f32 / 255.0;
                staged.push(horizon_client::gfx::PreparedModelDraw {
                    cpu_vertices: cut,
                    generation: copy.generation,
                    slot: copy.slot,
                    instance,
                });
                continue;
            }
            let Some(faces) = self.standalone_geometry.faces(copy.slot) else {
                continue;
            };
            let start = entities.len();
            super::minecarts::emit_model_faces(
                faces.iter().copied().map(|face| (face, copy.light)),
                world_geometry::row_major_3x4(copy.transform),
                true,
                |index| (index >= 0).then_some(copy.tint & 0xFF_FFFF),
                entities,
            );
            let shift = (DVec3::from(copy.origin) - camera).as_vec3();
            let alpha = copy.tint >> 24;
            for vertex in &mut entities[start..] {
                vertex.pos = (Vec3::from(vertex.pos) + shift).to_array();
                vertex.layer = copy.overlay;
                if alpha != 255 {
                    let scaled = (vertex.color >> 24) * alpha / 255;
                    vertex.color = vertex.color & 0x00FF_FFFF | scaled << 24;
                }
            }
        }
        draw.clear_copies();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use glam::{DVec3, Mat4, Quat, Vec3};
    use mod_api::{Identifier, LightCoords, ModelInstance, Overlay};

    fn rows(local: Mat4) -> [f32; 12] {
        let column = local.to_cols_array_2d();
        [
            column[0][0],
            column[1][0],
            column[2][0],
            column[3][0],
            column[0][1],
            column[1][1],
            column[2][1],
            column[3][1],
            column[0][2],
            column[1][2],
            column[2][2],
            column[3][2],
        ]
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES and software Vulkan"]
    fn draw_model_instances_matches_cpu_pixels() {
        let _fixture =
            horizon_client::models::standalone_fixture(vec!["minecraft:block/grass_block".into()]);
        super::super::native_entity_refs::tests::with_client_game(|game| {
            let model = game
                .native_model(&Identifier::parse("minecraft:block/grass_block").unwrap())
                .unwrap();
            let local = Mat4::from_scale(Vec3::splat(1.0));
            let origin = DVec3::new(8_000_000.5, 70.0, -4_000_000.25);
            let camera = DVec3::new(8_000_000.0, 70.2, -4_000_000.0);
            let instance = ModelInstance::new(
                origin.to_array(),
                rows(local),
                0xFF80_C040,
                LightCoords::new(3, 15).unwrap(),
                Overlay::NONE,
            );
            let mut draw = WorldGeometry::default();
            draw.reset(camera.to_array(), Mat4::IDENTITY, [0.0, 0.0]);
            game.native_world_draw_model_instances(&mut draw, model, &[instance])
                .unwrap();
            let copy = draw.copy(0);
            let mut entities = Vec::new();
            game.commit_model_instances(&mut draw, camera, &mut entities);
            assert!(!entities.is_empty(), "the public call drew nothing");
            let atlas = &game.gui.atlas;
            let pixels = atlas.pixels().to_vec();
            let view = Mat4::from_scale_rotation_translation(
                Vec3::new(1.4, 1.4, 0.2),
                Quat::IDENTITY,
                Vec3::new(-0.55, -0.85, 0.5),
            );
            let cpu = horizon_client::gfx::entity_pixel_test::render(
                &entities,
                [atlas.width, atlas.height],
                &pixels,
                view,
                false,
            )
            .expect("software Vulkan entity pixels");
            let (vertices, ranges) = game.standalone_geometry.prepared_upload();
            let gpu = horizon_client::gfx::entity_pixel_test::render_instance(
                horizon_client::gfx::entity_pixel_test::PreparedInstanceDraw {
                    vertices: &vertices,
                    ranges,
                    uploaded: copy.generation,
                    requested: copy.generation,
                    slot: copy.slot,
                    instances: std::slice::from_ref(
                        &super::super::native_block_entity_draw::prepared_instance(
                            world_geometry::row_major_3x4(copy.transform),
                            origin,
                            camera,
                            copy.tint,
                            copy.light,
                        ),
                    ),
                    per_copy: false,
                    hdr: false,
                    atlas_size: [atlas.width, atlas.height],
                    atlas: &pixels,
                    view_proj: view,
                },
            )
            .expect("software Vulkan prepared instance");
            let worst = cpu
                .iter()
                .zip(gpu.iter())
                .map(|(cpu, gpu)| cpu.abs_diff(*gpu))
                .max()
                .unwrap_or(0);
            assert!(
                worst <= 2,
                "public instance and cpu pixels differed by {worst}"
            );
            assert!(
                cpu.as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0),
                "the model drew no colour"
            );
        });
    }
}
