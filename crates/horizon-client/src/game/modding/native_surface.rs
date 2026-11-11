// game-module-visibility: pub(crate)
//! The `friction` Part method (`IBlockExtension.getFriction`). The client library asks through
//! `native_client_hooks`; only a state whose block has a subscribing part is converted.
use super::api_values::{TryIntoApi, block_state};
use super::{Game, boats::Boat};
use horizon_core::position::BlockPos;
use horizon_world::{native_load_context::LoopContext, registry::BlockStateId};

/// The game's side of `horizon_client::motion::native_friction`.
pub(crate) fn friction(
    context: &LoopContext,
    entity: u32,
    kind: &str,
    pos: BlockPos,
    state: BlockStateId,
    friction: &mut f32,
) {
    if !block_state::block_listens(mod_api::HookId::Friction, state) {
        return;
    }
    let Ok(state) = state.try_into_api() else {
        return;
    };
    mod_api::dispatch_friction(
        &horizon_client_render::api_context::LoopReader(context).context(),
        entity,
        kind,
        pos,
        state,
        friction,
    );
}

impl Game<'_> {
    /// The friction of `state` at `pos` under the boat `b` (`AbstractBoat.getGroundFriction`,
    /// `Leashable.angularFriction`), from `own`, the block's value. A boat's type is a built
    /// name, so it is built only when the block has a part to answer.
    pub(super) fn boat_friction(
        &self,
        b: &Boat,
        pos: BlockPos,
        state: BlockStateId,
        own: f32,
    ) -> f32 {
        if !block_state::block_listens(mod_api::HookId::Friction, state) {
            return own;
        }
        let mut friction = own;
        let kind = b.entity_type();
        horizon_client::motion::native_friction(
            &self.loop_context,
            b.id,
            &kind,
            pos,
            state,
            &mut friction,
        );
        friction
    }
}

#[cfg(test)]
mod tests {
    use super::super::Game;
    use super::super::native_entity_refs::tests::with_game;
    use horizon_client::entities::{ArmorStand, StandMedium};
    use horizon_client::motion::{MotionWorld, PlayerMotion};
    use horizon_core::{geometry::Vec3, position::BlockPos};
    use horizon_world::living_air::LivingAirWorld;

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_block_part_answers_the_friction_an_entity_reads() {
        // The fixture's parts: glass is 0.98 to a player only, white wool 0.98 to every entity,
        // red wool NaN, blue wool 3.0, green wool infinity and black wool -0.5.
        mod_api::__private::part_tests::friction_game_thread();
        // The client library asks the mods through the executable's callbacks.
        crate::entry::install_client_hooks();
        with_game(|game| {
            game.world.chunks.insert(
                (0, 0),
                std::sync::Arc::new(horizon_client::chunk::Chunk::empty(0, 0)),
            );
            let floor = BlockPos::new(5, 64, 5);
            // The player's ground travel reads the block through its motion world.
            let player = |game: &mut Game<'_>| {
                let mut player = PlayerMotion::new(game.motion, Vec3::new(5.5, 65.0, 5.5)).unwrap();
                let mut world = MotionWorld {
                    entities: &[],
                    player: &mut player,
                    scene: &mut game.world,
                    assets: &game.pack.assets,
                    collision_binding: game.collision,
                    resources: game.motion,
                    health: &game.health_resources,
                    tag_items: Some(game.res.items()),
                    loop_context: &game.loop_context,
                    damage_hooks: &game.damage_hooks,
                    block_registries: Some(&game.ui.registries),
                };
                LivingAirWorld::block_friction(&mut world, floor).unwrap()
            };
            // An armor stand moving 0.2 along x on the ground, after one travel: 0.2 × friction × 0.91.
            let stand = |game: &Game<'_>| {
                let mut stand = ArmorStand::new(7, Vec3::new(5.5, 65.0, 5.5), 0.0);
                stand.on_ground = true;
                stand.velocity = Vec3::new(0.2, 0.0, 0.0);
                let mut scene = horizon_client::collisions::SceneCollisions {
                    world: &game.world,
                    binding: game.collision,
                    subject: None,
                };
                let step = stand
                    .tick_move(
                        &mut scene,
                        game.res.facts(),
                        &game.motion.climbing,
                        false,
                        None,
                        &[],
                        &game.loop_context,
                    )
                    .unwrap();
                stand.tick_settle(StandMedium::Air, step);
                stand.velocity.x
            };
            let item = |game: &Game<'_>| {
                horizon_client::motion::block_friction(
                    &game.loop_context,
                    8,
                    "minecraft:item",
                    &game.world,
                    game.res.facts(),
                    floor,
                )
            };
            let registry = game.pack.assets.registry;
            for (block, player_friction, stand_x, item_friction) in [
                // No part: the block's own value, for every entity.
                ("minecraft:stone", 0.6, 0.10920001, 0.6),
                ("minecraft:ice", 0.98, 0.17836001, 0.98),
                // A part answering for the player only.
                ("minecraft:glass", 0.98, 0.10920001, 0.6),
                // A part answering for every entity.
                ("minecraft:white_wool", 0.98, 0.17836001, 0.98),
                // Answers out of range are dropped for the block's own.
                ("minecraft:red_wool", 0.6, 0.10920001, 0.6),
                ("minecraft:blue_wool", 0.6, 0.10920001, 0.6),
                ("minecraft:green_wool", 0.6, 0.10920001, 0.6),
                ("minecraft:black_wool", 0.6, 0.10920001, 0.6),
            ] {
                let state = registry.default_state(block).unwrap();
                game.world.set(floor.x, floor.y, floor.z, state).unwrap();
                assert_eq!(player(game), player_friction, "{block}");
                assert!((stand(game) - stand_x).abs() < 1.0e-7, "{block}");
                assert_eq!(item(game), item_friction, "{block}");
            }
        });
    }
}
