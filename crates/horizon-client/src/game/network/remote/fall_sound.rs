use super::*;

impl Game<'_> {
    pub(super) fn block_fall(&mut self, event: horizon_client::landing::FallEvent) {
        let (position, state) = match event {
            horizon_client::landing::FallEvent::Sound {
                sound,
                volume,
                pitch,
                ..
            } => {
                if let Some(name) = sound
                    .player_fall_sound()
                    .or_else(|| sound.honey_landing_sound())
                {
                    self.play_client_player_sound(name, volume, pitch);
                }
                return;
            }
            horizon_client::landing::FallEvent::BlockFallSound { position, state } => {
                (position, state)
            }
            _ => return,
        };
        let sound = self.own_sound_type(
            [position.x, position.y, position.z],
            state,
            mod_api::BlockSoundCause::Fall,
        );
        self.play_client_player_sound(&sound.fall, sound.volume * 0.5, sound.pitch * 0.75);
    }
}

#[cfg(test)]
#[path = "fall_sound_tests.rs"]
mod tests;
