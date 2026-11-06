//! The game's side of the `entities` area (`horizon-mod-api/src/host/entities.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl EntitiesHost for Game<'_> {
    fn entity(&self, id: u32) -> Option<EntityRef> {
        mod_api::__private::check_access("entity", mod_api::__private::LoadAccess::WorldOnly);
        self.native_entity(id)
    }
    fn level(&self, dimension: &str) -> Option<LevelRef> {
        mod_api::__private::check_access("level", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level(dimension)
    }
}
impl EntitiesServerHost for Game<'_> {
    fn summon(&mut self, dimension: &str, kind: &str, pos: [f64; 3], nbt: Compound) -> Option<u32> {
        mod_api::__private::check_access("summon", mod_api::__private::LoadAccess::WorldOnly);
        self.native_summon(dimension, kind, pos, nbt)
    }
    fn set_experience_orb(
        &mut self,
        dimension: &str,
        id: u32,
        value: Option<u32>,
        count: Option<u32>,
        age: Option<u32>,
    ) -> bool {
        mod_api::__private::check_access(
            "set_experience_orb",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_set_experience_orb(dimension, id, value, count, age)
    }
    fn experience_orbs(
        &self,
        dimension: &str,
        min: [f64; 3],
        max: [f64; 3],
    ) -> Vec<NativeExperienceOrb> {
        mod_api::__private::check_access(
            "experience_orbs",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_experience_orbs(dimension, min, max)
    }
    fn spawn_falling_block(
        &mut self,
        dimension: &str,
        state: BlockStateId,
        pos: [f64; 3],
        velocity: [f64; 3],
        falling: NativeFalling,
    ) -> bool {
        mod_api::__private::check_access(
            "spawn_falling_block",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_spawn_falling_block(dimension, state, pos, velocity, falling)
    }
}
impl EntitiesBackend for Game<'_> {
    fn entity_tags(&self, entity: EntityRef) -> Option<Vec<String>> {
        mod_api::__private::check_access("entity_tags", mod_api::__private::LoadAccess::WorldOnly);
        self.native_entity_tags(entity)
    }
    fn entity_name(&self, entity: EntityRef) -> Option<String> {
        mod_api::__private::check_access("entity_name", mod_api::__private::LoadAccess::WorldOnly);
        self.native_entity_name(entity)
    }
    fn entity_custom_name_visible(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_custom_name_visible",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_custom_name_visible(entity)
    }
    fn entity_classification(&self, entity: EntityRef) -> Option<MobCategory> {
        mod_api::__private::check_access(
            "entity_classification",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_classification(entity)
    }
    fn entity_added_to_level(&self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_added_to_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_added_to_level(entity)
    }
    fn entity_position(&self, entity: EntityRef) -> Option<[f64; 3]> {
        mod_api::__private::check_access(
            "entity_position",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_position(entity)
    }
    fn entity_block_position(&self, entity: EntityRef) -> Option<[i32; 3]> {
        mod_api::__private::check_access(
            "entity_block_position",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_block_position(entity)
    }
    fn entity_rotation(&self, entity: EntityRef) -> Option<[f32; 2]> {
        mod_api::__private::check_access(
            "entity_rotation",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_rotation(entity)
    }
    fn entity_kind(&self, entity: EntityRef) -> Option<String> {
        mod_api::__private::check_access("entity_kind", mod_api::__private::LoadAccess::WorldOnly);
        self.native_entity_kind(entity)
    }
    fn entity_level(&self, entity: EntityRef) -> Option<LevelRef> {
        mod_api::__private::check_access("entity_level", mod_api::__private::LoadAccess::WorldOnly);
        self.native_entity_level(entity)
    }
    fn entity_permanently_invulnerable(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_permanently_invulnerable",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_permanently_invulnerable(entity)
    }
    fn entity_damage_cooldown_time(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_damage_cooldown_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_damage_cooldown_time(entity)
    }
    fn entity_velocity(&self, entity: EntityRef) -> Option<[f64; 3]> {
        mod_api::__private::check_access(
            "entity_velocity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_velocity(entity)
    }
    fn entity_on_ground(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_on_ground",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_on_ground(entity)
    }
    fn entity_bounding_box(&self, entity: EntityRef) -> Option<[[f64; 3]; 2]> {
        mod_api::__private::check_access(
            "entity_bounding_box",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_bounding_box(entity)
    }
    fn entity_eye_height(&self, entity: EntityRef) -> Option<f64> {
        mod_api::__private::check_access(
            "entity_eye_height",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_eye_height(entity)
    }
    fn entity_eye_position(&self, entity: EntityRef) -> Option<[f64; 3]> {
        mod_api::__private::check_access(
            "entity_eye_position",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_eye_position(entity)
    }
    fn entity_view_vector(&self, entity: EntityRef) -> Option<[f64; 3]> {
        mod_api::__private::check_access(
            "entity_view_vector",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_view_vector(entity)
    }
    fn entity_distance_to_sqr(&self, entity: EntityRef, pos: [f64; 3]) -> Option<f64> {
        mod_api::__private::check_access(
            "entity_distance_to_sqr",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_distance_to_sqr(entity, pos)
    }
    fn entity_head_yaw(&self, entity: EntityRef) -> Option<f32> {
        mod_api::__private::check_access(
            "entity_head_yaw",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_head_yaw(entity)
    }
    fn entity_direction(&self, entity: EntityRef) -> Option<&'static str> {
        mod_api::__private::check_access(
            "entity_direction",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_direction(entity)
    }
    fn entity_dimensions(&self, entity: EntityRef) -> Option<[f64; 2]> {
        mod_api::__private::check_access(
            "entity_dimensions",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_dimensions(entity)
    }
    fn entity_no_physics(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_no_physics",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_no_physics(entity)
    }
    fn entity_vehicle(&self, entity: EntityRef) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "entity_vehicle",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_vehicle(entity)
    }
    fn entity_is_passenger(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_passenger",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_passenger(entity)
    }
    fn entity_root_vehicle(&self, entity: EntityRef) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "entity_root_vehicle",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_root_vehicle(entity)
    }
    fn entity_passengers(&self, entity: EntityRef) -> Option<Vec<EntityRef>> {
        mod_api::__private::check_access(
            "entity_passengers",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_passengers(entity)
    }
    fn entity_is_vehicle(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_vehicle",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_vehicle(entity)
    }
    fn entity_sound_source(&self, entity: EntityRef) -> Option<SoundSource> {
        mod_api::__private::check_access(
            "entity_sound_source",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_sound_source(entity)
    }
    fn entity_spectator(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_spectator",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_spectator(entity)
    }
    fn entity_is_shift_key_down(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_shift_key_down",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_shift_key_down(entity)
    }
    fn entity_is_crouching(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_crouching",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_crouching(entity)
    }
    fn entity_is_sprinting(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_sprinting",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_sprinting(entity)
    }
    fn entity_is_in_water(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_in_water",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_in_water(entity)
    }
    fn entity_is_in_lava(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_in_lava",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_in_lava(entity)
    }
    fn entity_fire_immune(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_fire_immune",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_fire_immune(entity)
    }
    fn entity_fire_ticks(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_fire_ticks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_fire_ticks(entity)
    }
    fn entity_portal_cooldown(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_portal_cooldown",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_portal_cooldown(entity)
    }
    fn entity_air_supply(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_air_supply",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_air_supply(entity)
    }
    fn entity_max_air_supply(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_max_air_supply",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_max_air_supply(entity)
    }
    fn entity_is_silent(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_silent",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_silent(entity)
    }
    fn entity_glowing(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_glowing",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_glowing(entity)
    }
    fn entity_no_gravity(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_no_gravity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_no_gravity(entity)
    }
    fn entity_ticks_frozen(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_ticks_frozen",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_ticks_frozen(entity)
    }
    fn entity_tick_count(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_tick_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_tick_count(entity)
    }
    fn entity_is_invisible(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_invisible",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_invisible(entity)
    }
    fn entity_pose(&self, entity: EntityRef) -> Option<&'static str> {
        mod_api::__private::check_access("entity_pose", mod_api::__private::LoadAccess::WorldOnly);
        self.native_entity_pose(entity)
    }
}
impl EntitiesBackendMut for Game<'_> {
    fn entity_nbt(&mut self, entity: EntityRef) -> Option<Compound> {
        mod_api::__private::check_access("entity_nbt", mod_api::__private::LoadAccess::WorldOnly);
        self.native_entity_nbt(entity)
    }
    fn entity_merge_nbt(&mut self, entity: EntityRef, nbt: Compound) -> bool {
        mod_api::__private::check_access(
            "entity_merge_nbt",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_merge_nbt(entity, nbt)
    }
    fn entity_add_tag(&mut self, entity: EntityRef, tag: &str) -> bool {
        mod_api::__private::check_access(
            "entity_add_tag",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_add_tag(entity, tag)
    }
    fn entity_remove_tag(&mut self, entity: EntityRef, tag: &str) -> bool {
        mod_api::__private::check_access(
            "entity_remove_tag",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_remove_tag(entity, tag)
    }
    fn entity_set_custom_name(&mut self, entity: EntityRef, name: Option<Text>) -> bool {
        mod_api::__private::check_access(
            "entity_set_custom_name",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_custom_name(entity, name)
    }
    fn entity_set_custom_name_visible(&mut self, entity: EntityRef, visible: bool) -> bool {
        mod_api::__private::check_access(
            "entity_set_custom_name_visible",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_custom_name_visible(entity, visible)
    }
    fn entity_reset_fall_distance(&mut self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_reset_fall_distance",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_reset_fall_distance(entity)
    }
    fn entity_uuid(&mut self, entity: EntityRef) -> Option<u128> {
        mod_api::__private::check_access("entity_uuid", mod_api::__private::LoadAccess::WorldOnly);
        self.native_entity_uuid(entity)
    }
    fn entity_set_invulnerable(&mut self, entity: EntityRef, invulnerable: bool) -> bool {
        mod_api::__private::check_access(
            "entity_set_invulnerable",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_invulnerable(entity, invulnerable)
    }
    fn entity_set_damage_cooldown_time(&mut self, entity: EntityRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "entity_set_damage_cooldown_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_damage_cooldown_time(entity, ticks)
    }
    fn entity_set_velocity(&mut self, entity: EntityRef, velocity: [f64; 3]) -> bool {
        mod_api::__private::check_access(
            "entity_set_velocity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_velocity(entity, velocity)
    }
    fn entity_teleport_to(
        &mut self,
        entity: EntityRef,
        pos: [f64; 3],
        rotation: Option<[f32; 2]>,
    ) -> bool {
        mod_api::__private::check_access(
            "entity_teleport_to",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_teleport_to(entity, pos, rotation)
    }
    fn entity_set_rotation(&mut self, entity: EntityRef, yaw: f32, pitch: f32) -> bool {
        mod_api::__private::check_access(
            "entity_set_rotation",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_rotation(entity, yaw, pitch)
    }
    fn entity_set_head_yaw(&mut self, entity: EntityRef, yaw: f32) -> bool {
        mod_api::__private::check_access(
            "entity_set_head_yaw",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_head_yaw(entity, yaw)
    }
    fn entity_push(&mut self, entity: EntityRef, delta: [f64; 3]) -> bool {
        mod_api::__private::check_access("entity_push", mod_api::__private::LoadAccess::WorldOnly);
        self.native_entity_push(entity, delta)
    }
    fn entity_discard(&mut self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_discard",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_discard(entity)
    }
    fn entity_remove(&mut self, entity: EntityRef, reason: NativeRemovalReason) -> bool {
        mod_api::__private::check_access(
            "entity_remove",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_remove(entity, reason)
    }
    fn entity_spawn_at_location(
        &mut self,
        entity: EntityRef,
        stack: ItemStack,
        y_offset: f32,
    ) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "entity_spawn_at_location",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_spawn_at_location(entity, stack, y_offset)
    }
    fn entity_start_riding(&mut self, entity: EntityRef, vehicle: EntityRef, force: bool) -> bool {
        mod_api::__private::check_access(
            "entity_start_riding",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_start_riding(entity, vehicle, force)
    }
    fn entity_stop_riding(&mut self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_stop_riding",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_stop_riding(entity)
    }
    fn entity_eject_passengers(&mut self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_eject_passengers",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_eject_passengers(entity)
    }
    fn entity_play_sound(
        &mut self,
        entity: EntityRef,
        sound: &str,
        volume: f32,
        pitch: f32,
    ) -> bool {
        mod_api::__private::check_access(
            "entity_play_sound",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_play_sound(entity, sound, volume, pitch)
    }
    fn entity_game_event(&mut self, entity: EntityRef, event: &str) -> bool {
        mod_api::__private::check_access(
            "entity_game_event",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_game_event(entity, event)
    }
    fn entity_broadcast_event(&mut self, entity: EntityRef, id: u8) -> bool {
        mod_api::__private::check_access(
            "entity_broadcast_event",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_broadcast_event(entity, id)
    }
    fn entity_set_remaining_fire_ticks(&mut self, entity: EntityRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "entity_set_remaining_fire_ticks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_remaining_fire_ticks(entity, ticks)
    }
    fn entity_clear_fire(&mut self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_clear_fire",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_clear_fire(entity)
    }
    fn entity_set_air_supply(&mut self, entity: EntityRef, air: i32) -> bool {
        mod_api::__private::check_access(
            "entity_set_air_supply",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_air_supply(entity, air)
    }
    fn entity_set_silent(&mut self, entity: EntityRef, silent: bool) -> bool {
        mod_api::__private::check_access(
            "entity_set_silent",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_silent(entity, silent)
    }
    fn entity_set_glowing(&mut self, entity: EntityRef, glowing: bool) -> bool {
        mod_api::__private::check_access(
            "entity_set_glowing",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_glowing(entity, glowing)
    }
    fn entity_set_no_gravity(&mut self, entity: EntityRef, no_gravity: bool) -> bool {
        mod_api::__private::check_access(
            "entity_set_no_gravity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_no_gravity(entity, no_gravity)
    }
    fn entity_set_ticks_frozen(&mut self, entity: EntityRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "entity_set_ticks_frozen",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_ticks_frozen(entity, ticks)
    }
    fn entity_set_invisible(&mut self, entity: EntityRef, invisible: bool) -> bool {
        mod_api::__private::check_access(
            "entity_set_invisible",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_invisible(entity, invisible)
    }
    fn entity_play_bound_sound(
        &mut self,
        entity: EntityRef,
        sound: &str,
        source: SoundSource,
        volume: f32,
        pitch: f32,
    ) -> bool {
        mod_api::__private::check_access(
            "entity_play_bound_sound",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_play_bound_sound(entity, sound, source, volume, pitch)
    }
}
impl EntitiesClientBackendMut for Game<'_> {
    fn entity_sprint_particles(&mut self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_sprint_particles",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_sprint_particles(entity)
    }
}
