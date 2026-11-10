use super::*;
use crate::game::Dim;
use horizon_client::mesher::Vertex;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// Draw in one first-person hand of this window's player, once a frame
    /// for each hand `native_render_hand` is asked about, right after it
    /// (`RenderHandEvent`'s pose stack and submit collector): a hand that
    /// holds an item or a map, the empty main hand's arm, and the empty off
    /// hand, where vanilla draws nothing. `native_render_arm` only covers
    /// vanilla's arms. `render` is what `native_render_hand` got, with the
    /// pose the mods set, and `canceled` says whether they left vanilla's
    /// hand out. `draw`'s pose stack starts in the hands' view space, in
    /// blocks, +x right, +y up, -z into the screen, with the hand sway and
    /// that pose applied and none of vanilla's per-item transforms; both
    /// hands share it, and `render.hand` says which one is asked. It is lit
    /// with the light at the eyes; add geometry with `native_draw_cube`,
    /// `native_draw_quad`, `native_draw_block` and `native_draw_item`. It
    /// joins the hand's own vertices: the hand's field of view, the view
    /// bob, a shader pack's hand pass and `native_hand_drawn` see it as they
    /// see vanilla's hand. To replace a hand, cancel it in
    /// `native_render_hand` and draw here; what the mods drew stays.
    /// Client-side.
    pub(in crate::game) fn native_hand_submit(
        &self,
        render: &HandRender<'_>,
        canceled: bool,
        draw: &mut BlockEntityDraw,
    ) {
        self.fire_hand_submit(render, canceled, draw);
    }
}

impl Game<'_> {
    /// The hands' view space `to_world` under the pose the mods set on
    /// `render` (`native_render_hand`): `to_world` itself when no mod moved
    /// the hand, always so without one, with no matrices built.
    pub(in crate::game) fn posed_hand(render: &HandRender<'_>, to_world: glam::Mat4) -> glam::Mat4 {
        let [x, y, z] = render.translation;
        let [pitch, yaw, roll] = render.rotation;
        let pose = [x, y, z, pitch, yaw, roll, render.scale];
        if pose == [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0] {
            to_world
        } else {
            to_world * hand_pose_matrix(pose)
        }
    }

    /// Asks the mods for one hand's own geometry (`native_hand_submit`), in
    /// the hands' view space `to_world` under the pose `render` carries, and
    /// appends what they drew to the hand's vertices `out`. A hand a mod drew
    /// in starts a run of its own, with no rendered item id, as the arm does.
    /// With no listener it builds nothing.
    pub(in crate::game) fn hand_submit_geometry(
        &self,
        render: &HandRender<'_>,
        canceled: bool,
        to_world: glam::Mat4,
        light: u32,
        out: &mut Vec<Vertex>,
        runs: &mut Vec<(u32, i32)>,
    ) {
        if !mod_api::__private::listened(mod_api::HookId::HandSubmit) {
            return;
        }
        let to_world = Self::posed_hand(render, to_world);
        let start = out.len();
        let eye = self.eye_position().unwrap_or_default();
        let mut draw = BlockEntityDraw::posed(
            render.partial,
            self.world.game_time,
            light,
            eye.map(|c| c.floor() as i32),
            to_world,
            horizon_client::block_entity_render::diffuse_lights(self.dim == Dim::Nether),
            std::mem::take(out),
        );
        self.native_hand_submit(render, canceled, &mut draw);
        *out = draw.into_vertices();
        if out.len() > start {
            runs.push((start as u32, 0));
        }
    }
}
