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
            let (quads, _) = entities[start..].as_chunks_mut::<4>();
            for (face, quad) in faces.iter().zip(quads) {
                for vertex in quad.iter_mut() {
                    vertex.pos = (Vec3::from(vertex.pos) + shift).to_array();
                    vertex.layer = copy.overlay;
                }
                if alpha != 255 && face.tint_index >= 0 {
                    for vertex in quad.iter_mut() {
                        let scaled = (vertex.color >> 24) * alpha / 255;
                        vertex.color = vertex.color & 0x00FF_FFFF | scaled << 24;
                    }
                }
            }
        }
        draw.clear_copies();
    }
}
