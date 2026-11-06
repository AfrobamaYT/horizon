//! The game's side of the `levels` area (`horizon-mod-api/src/host/levels.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl LevelsHost for Game<'_> {
    fn is_loaded(&self, id: &str) -> bool {
        mod_api::__private::check_access("is_loaded", mod_api::__private::LoadAccess::WorldOnly);
        Self::native_is_loaded(id)
    }
    fn chunk(&self, level: LevelRef, pos: [i32; 2]) -> Option<ChunkRef> {
        mod_api::__private::check_access("chunk", mod_api::__private::LoadAccess::WorldOnly);
        self.native_chunk(level, pos)
    }
}
impl LevelsBackend for Game<'_> {
    fn dimension_key_id(&self, key: DimensionKey) -> Option<String> {
        mod_api::__private::check_access(
            "dimension_key_id",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_dimension_key_id(key)
    }
    fn level_chunk_ref(&self, level: LevelRef, pos: [i32; 2]) -> Option<ChunkRef> {
        mod_api::__private::check_access(
            "level_chunk_ref",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_chunk_ref(level, pos)
    }
    fn chunk_forced(&self, chunk: ChunkRef) -> Option<bool> {
        mod_api::__private::check_access("chunk_forced", mod_api::__private::LoadAccess::WorldOnly);
        self.native_chunk_forced(chunk)
    }
    fn level_forced_chunks(&self, level: LevelRef) -> Option<Vec<[i32; 2]>> {
        mod_api::__private::check_access(
            "level_forced_chunks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_forced_chunks(level)
    }
    fn level_mod_forced_chunks(
        &self,
        level: LevelRef,
        controller: TicketController,
    ) -> Option<Vec<(NativeTicketOwner, [i32; 2], bool)>> {
        mod_api::__private::check_access(
            "level_mod_forced_chunks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_mod_forced_chunks(level, controller)
    }
    fn chunk_loaded(&self, chunk: ChunkRef) -> bool {
        mod_api::__private::check_access("chunk_loaded", mod_api::__private::LoadAccess::WorldOnly);
        self.native_chunk_ref_loaded(chunk)
    }
    fn chunk_status(&self, chunk: ChunkRef) -> Option<ChunkStatus> {
        mod_api::__private::check_access("chunk_status", mod_api::__private::LoadAccess::WorldOnly);
        self.native_chunk_status(chunk)
    }
    fn level_craft(
        &self,
        level: LevelRef,
        width: u32,
        height: u32,
        items: &[Option<ItemStack>],
    ) -> Option<CraftResult> {
        mod_api::__private::check_access("level_craft", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_craft(level, width, height, items)
    }
    fn level_entities_in(
        &self,
        level: LevelRef,
        min: [f64; 3],
        max: [f64; 3],
    ) -> Option<Vec<EntityRef>> {
        mod_api::__private::check_access(
            "level_entities_in",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_entities_in(level, min, max)
    }
    fn level_block_state(&self, level: LevelRef, pos: [i32; 3]) -> Option<BlockStateId> {
        mod_api::__private::check_access(
            "level_block_state",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_state(level, pos)
    }
    fn level_block_transform(
        &self,
        level: LevelRef,
        player: PlayerRef,
        hand: Hand,
        stack: &ItemStack,
        pos: [i32; 3],
        hit: BlockHit,
    ) -> Option<BlockTransformPreview> {
        mod_api::__private::check_access(
            "level_block_transform",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_block_transform(level, player, hand, stack, pos, hit)
    }
    fn level_block(&self, level: LevelRef, pos: [i32; 3]) -> Option<String> {
        mod_api::__private::check_access("level_block", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_block(level, pos)
    }
    fn level_block_property(&self, level: LevelRef, pos: [i32; 3], key: &str) -> Option<String> {
        mod_api::__private::check_access(
            "level_block_property",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_block_property(level, pos, key)
    }
    fn level_in_place(&self, level: LevelRef) -> bool {
        mod_api::__private::check_access(
            "level_in_place",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_in_place(level)
    }
    fn level_is_loaded(&self, level: LevelRef, pos: [i32; 3]) -> bool {
        mod_api::__private::check_access(
            "level_is_loaded",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_is_loaded(level, pos)
    }
    fn level_brightness(&self, level: LevelRef, layer: LightLayer, pos: [i32; 3]) -> Option<i32> {
        mod_api::__private::check_access(
            "level_brightness",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_brightness(level, layer, pos)
    }
    fn level_height(&self, level: LevelRef) -> Option<(i32, i32)> {
        mod_api::__private::check_access("level_height", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_height(level)
    }
    fn level_game_time(&self, level: LevelRef) -> Option<i64> {
        mod_api::__private::check_access(
            "level_game_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_game_time(level)
    }
    fn level_day_time(&self, level: LevelRef) -> Option<i64> {
        mod_api::__private::check_access(
            "level_day_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_day_time(level)
    }
    fn level_fluid_flow(&self, level: LevelRef, pos: [i32; 3]) -> Option<Vec3> {
        mod_api::__private::check_access(
            "level_fluid_flow",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_fluid_flow(level, pos)
    }
    fn level_in_world_bounds(&self, level: LevelRef, pos: [i32; 3]) -> Option<bool> {
        mod_api::__private::check_access(
            "level_in_world_bounds",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_in_world_bounds(level, pos)
    }
    fn level_in_spawnable_bounds(&self, level: LevelRef, pos: [i32; 3]) -> Option<bool> {
        mod_api::__private::check_access(
            "level_in_spawnable_bounds",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_in_spawnable_bounds(level, pos)
    }
    fn level_block_shape(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        kind: NativeBlockShapeKind,
    ) -> Option<Vec<[f64; 6]>> {
        mod_api::__private::check_access(
            "level_block_shape",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_block_shape(level, pos, kind)
    }
    fn level_clip(
        &self,
        level: LevelRef,
        from: [f64; 3],
        to: [f64; 3],
        blocks: NativeClipBlock,
        fluids: NativeClipFluid,
    ) -> Option<NativeClipHit> {
        mod_api::__private::check_access("level_clip", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_clip(level, from, to, blocks, fluids)
    }
    fn level_block_face_sturdy(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        face: NativeBlockFace,
        support: NativeSupportType,
    ) -> Option<bool> {
        mod_api::__private::check_access(
            "level_block_face_sturdy",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_block_face_sturdy(level, pos, face, support)
    }
    fn level_signal(&self, level: LevelRef, pos: [i32; 3], direction: Direction) -> Option<i32> {
        mod_api::__private::check_access("level_signal", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_signal(level, pos, direction)
    }
    fn level_direct_signal(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        direction: Direction,
    ) -> Option<i32> {
        mod_api::__private::check_access(
            "level_direct_signal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_direct_signal(level, pos, direction)
    }
    fn level_direct_signal_to(&self, level: LevelRef, pos: [i32; 3]) -> Option<i32> {
        mod_api::__private::check_access(
            "level_direct_signal_to",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_direct_signal_to(level, pos)
    }
    fn level_best_neighbor_signal(&self, level: LevelRef, pos: [i32; 3]) -> Option<i32> {
        mod_api::__private::check_access(
            "level_best_neighbor_signal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_best_neighbor_signal(level, pos)
    }
    fn level_has_neighbor_signal(&self, level: LevelRef, pos: [i32; 3]) -> bool {
        mod_api::__private::check_access(
            "level_has_neighbor_signal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_has_neighbor_signal(level, pos)
    }
    fn level_dimension_shape(&self, level: LevelRef) -> Option<LevelDimensionShape> {
        mod_api::__private::check_access(
            "level_dimension_shape",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_dimension_shape(level)
    }
    fn level_seed(&self, level: LevelRef) -> Option<i64> {
        mod_api::__private::check_access("level_seed", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_seed(level)
    }
    fn level_difficulty(&self, level: LevelRef) -> Option<&'static str> {
        mod_api::__private::check_access(
            "level_difficulty",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_difficulty(level)
    }
    fn level_game_rule_bool(&self, level: LevelRef, key: BoolGameRuleKey) -> Option<bool> {
        mod_api::__private::check_access(
            "level_game_rule_bool",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_game_rule_bool(level, key)
    }
    fn level_game_rule(&self, level: LevelRef, rule: &str) -> Option<GameRuleValue> {
        mod_api::__private::check_access(
            "level_game_rule",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_game_rule(level, rule)
    }
    fn level_game_rules(&self, level: LevelRef) -> Option<Vec<(String, GameRuleValue)>> {
        mod_api::__private::check_access(
            "level_game_rules",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_game_rules(level)
    }
    fn level_weather(&self, level: LevelRef) -> Option<(f32, f32)> {
        mod_api::__private::check_access(
            "level_weather",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_weather(level)
    }
    fn level_is_raining(&self, level: LevelRef) -> Option<bool> {
        mod_api::__private::check_access(
            "level_is_raining",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_is_raining(level)
    }
    fn level_is_raining_at(&self, level: LevelRef, pos: [i32; 3]) -> Option<bool> {
        mod_api::__private::check_access(
            "level_is_raining_at",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_is_raining_at(level, pos)
    }
    fn level_enabled_features(&self, level: LevelRef) -> Option<FeatureFlags> {
        mod_api::__private::check_access(
            "level_enabled_features",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_enabled_features(level)
    }
    fn level_is_thundering(&self, level: LevelRef) -> Option<bool> {
        mod_api::__private::check_access(
            "level_is_thundering",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_is_thundering(level)
    }
    fn level_random(&self, level: LevelRef) -> Option<LegacyRandom> {
        mod_api::__private::check_access("level_random", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_random(level)
    }
    fn level_raid(&self, level: LevelRef, id: i32) -> Option<RaidRef> {
        mod_api::__private::check_access("level_raid", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_raid(level, id)
    }
    fn level_raid_at(&self, level: LevelRef, pos: [i32; 3]) -> Option<RaidRef> {
        mod_api::__private::check_access(
            "level_raid_at",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_raid_at(level, pos)
    }
    fn level_raids(&self, level: LevelRef) -> Option<Vec<RaidRef>> {
        mod_api::__private::check_access("level_raids", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_raids(level)
    }
    fn level_fluid_state(&self, level: LevelRef, pos: [i32; 3]) -> Option<FluidState> {
        mod_api::__private::check_access(
            "level_fluid_state",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_fluid_state(level, pos)
    }
}
impl LevelsBackendMut for Game<'_> {
    fn chunk_set_forced(&mut self, chunk: ChunkRef, forced: bool) -> bool {
        mod_api::__private::check_access(
            "chunk_set_forced",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_chunk_set_forced(chunk, forced)
    }
    fn chunk_set_mod_forced(
        &mut self,
        chunk: ChunkRef,
        controller: TicketController,
        owner: NativeTicketOwner,
        forced: bool,
        natural_spawning: bool,
    ) -> bool {
        mod_api::__private::check_access(
            "chunk_set_mod_forced",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_chunk_set_mod_forced(chunk, controller, owner, forced, natural_spawning)
    }
    fn level_set_block(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        state: BlockStateId,
        flags: NativeBlockUpdateFlags,
    ) -> bool {
        mod_api::__private::check_access(
            "level_set_block",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_set_block(level, pos, state, flags)
    }
    fn level_remove_block(&mut self, level: LevelRef, pos: [i32; 3], moved: bool) -> bool {
        mod_api::__private::check_access(
            "level_remove_block",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_remove_block(level, pos, moved)
    }
    fn level_destroy_block(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        drop: bool,
        tool: Option<&ItemStack>,
        collect: bool,
    ) -> Option<Vec<ItemStack>> {
        mod_api::__private::check_access(
            "level_destroy_block",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_destroy_block(level, pos, drop, tool, collect)
    }
    fn level_set_break_progress(
        &mut self,
        level: LevelRef,
        breaker: i32,
        pos: [i32; 3],
        progress: i32,
    ) -> bool {
        mod_api::__private::check_access(
            "level_set_break_progress",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_set_break_progress(level, breaker, pos, progress)
    }
    fn level_play_sound(
        &mut self,
        level: LevelRef,
        sound: &str,
        at: [f64; 3],
        source: SoundSource,
        volume: f32,
        pitch: f32,
    ) -> bool {
        mod_api::__private::check_access(
            "level_play_sound",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_play_sound(level, sound, at, source, volume, pitch)
    }
    fn level_send_particles(
        &mut self,
        level: LevelRef,
        options: &ParticleOptions,
        state: Option<BlockStateId>,
        at: [f64; 3],
        spread: [f64; 3],
        speed: f64,
        count: i32,
    ) -> bool {
        mod_api::__private::check_access(
            "level_send_particles",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_send_particles(level, options, state, at, spread, speed, count)
    }
    fn level_level_event(&mut self, level: LevelRef, id: i32, pos: [i32; 3], data: i32) -> bool {
        mod_api::__private::check_access(
            "level_level_event",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_level_event(level, id, pos, data)
    }
    fn level_game_event(&mut self, level: LevelRef, event: &str, at: [f64; 3]) -> bool {
        mod_api::__private::check_access(
            "level_game_event",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_game_event(level, event, at)
    }
    fn level_explode(
        &mut self,
        level: LevelRef,
        center: [f64; 3],
        radius: f32,
        fire: bool,
        interaction: NativeExplosionInteraction,
    ) -> bool {
        mod_api::__private::check_access(
            "level_explode",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_explode(level, center, radius, fire, interaction)
    }
    fn level_strike_lightning(&mut self, level: LevelRef, at: [f64; 3], visual_only: bool) -> bool {
        mod_api::__private::check_access(
            "level_strike_lightning",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_strike_lightning(level, at, visual_only)
    }
    fn level_pop_resource(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        stack: ItemStack,
    ) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "level_pop_resource",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_pop_resource(level, pos, stack)
    }
    fn level_spawn_item(
        &mut self,
        level: LevelRef,
        x: f64,
        y: f64,
        z: f64,
        stack: ItemStack,
        pickup_delay: u32,
    ) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "level_spawn_item",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_spawn_item(level, x, y, z, stack, pickup_delay)
    }
    fn level_spawn_item_with_motion(
        &mut self,
        level: LevelRef,
        x: f64,
        y: f64,
        z: f64,
        stack: ItemStack,
        motion: [f64; 3],
        pickup_delay: u32,
    ) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "level_spawn_item_with_motion",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_spawn_item_with_motion(level, x, y, z, stack, motion, pickup_delay)
    }
    fn level_award_experience(
        &mut self,
        level: LevelRef,
        x: f64,
        y: f64,
        z: f64,
        amount: u32,
    ) -> bool {
        mod_api::__private::check_access(
            "level_award_experience",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_award_experience(level, x, y, z, amount)
    }
    fn level_update_neighbors(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        source: BlockStateId,
        update: NeighbourUpdate,
    ) -> bool {
        mod_api::__private::check_access(
            "level_update_neighbors",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_update_neighbors(level, pos, source, update)
    }
    fn level_neighbor_changed(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        source: BlockStateId,
    ) -> bool {
        mod_api::__private::check_access(
            "level_neighbor_changed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_neighbor_changed(level, pos, source)
    }
    fn level_set_random(&mut self, level: LevelRef, random: LegacyRandom) -> bool {
        mod_api::__private::check_access(
            "level_set_random",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_set_random(level, random)
    }
    fn level_bone_meal(&mut self, level: LevelRef, pos: [i32; 3]) -> bool {
        mod_api::__private::check_access(
            "level_bone_meal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_bone_meal(level, pos)
    }
    fn level_set_biome(&mut self, level: LevelRef, pos: [i32; 3], biome: &str) -> bool {
        mod_api::__private::check_access(
            "level_set_biome",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_set_biome(level, pos, biome)
    }
}
