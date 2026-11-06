//! The game's side of the `level_world` area (`horizon-mod-api/src/host/level_world.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl LevelWorldBackend for Game<'_> {
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
}
impl LevelWorldBackendMut for Game<'_> {
    fn level_set_random(&mut self, level: LevelRef, random: LegacyRandom) -> bool {
        mod_api::__private::check_access(
            "level_set_random",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_set_random(level, random)
    }
}
