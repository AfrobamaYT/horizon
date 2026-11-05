//! The server's run/stop boundary, shared by integrated and dedicated servers.

use super::Game;
use horizon_client::Result;
use std::panic::{AssertUnwindSafe, catch_unwind};

impl Game<'_> {
    /// `MinecraftServer.runServer`'s finally: once startup begins, every exit
    /// stops and saves the server, including a failed init or tick.
    pub(super) fn run_server_lifecycle(
        &mut self,
        dedicated: bool,
        run: impl FnOnce(&mut Self, &mut bool) -> Result<()>,
    ) -> Result<()> {
        let mut startup_began = false;
        let run = catch_unwind(AssertUnwindSafe(|| run(self, &mut startup_began)));
        let crashed = run.is_err();
        let result = match run {
            Ok(result) => result,
            Err(payload) => {
                self.prepare_server_crash();
                Err(format!(
                    "the server thread panicked: {}",
                    super::dedicated::panic_message(payload.as_ref())
                )
                .into())
            }
        };
        if !startup_began {
            return result;
        }
        // Listeners still see the loaded worlds and players, even on failure.
        self.native_server_stopping(dedicated);
        if let Err(error) = &result {
            self.native_lifecycle_world_failed(&error.to_string());
            self.native_lifecycle_close_failed(&error.to_string());
        }
        let stopped = if dedicated {
            self.stop_dedicated(crashed)
        } else {
            self.stop_integrated(crashed)
        };
        if let Err(error) = &stopped {
            self.native_lifecycle_close_failed(&error.to_string());
            log::error!("Exception stopping the server: {error}");
        }
        self.native_server_stopped(dedicated);
        result.and(stopped)
    }

    /// A panic can interrupt `as_guest` with the player's current state swapped
    /// into Game. Keep that player's last saved file rather than an older guest
    /// record. If the network tick lost the hub, only world state can be saved.
    fn prepare_server_crash(&mut self) {
        let acting = self.lan_acting;
        if let (Some(conn), Some(host)) = (acting, self.lan.as_mut())
            && let Some(guest) = host.hub.guests.iter_mut().find(|g| g.conn == conn)
        {
            guest.placed = false;
        }
        if self.lan.is_none() {
            log::warn!(
                "The panic interrupted the network tick: players are not saved, only the world"
            );
        } else if acting.is_some() {
            log::warn!(
                "The panic interrupted a player's turn: that player keeps the last saved file"
            );
        }
    }

    /// Complete teardown even if freeing beds or the final save fails. A failed
    /// open must never overwrite a world the server did not successfully adopt.
    pub(super) fn finish_server_save(&mut self, crashed: bool) -> Result<()> {
        let beds = if crashed {
            Ok(())
        } else {
            self.free_player_beds()
        };
        let saved = if self.session_lock.is_some() {
            self.save_game()
        } else {
            Ok(())
        };
        self.lan_close(false);
        let flushed = self.save.flush();
        let result = beds.and(saved).and(flushed);
        if result.is_ok() {
            self.native_lifecycle_world_closed();
        }
        self.session_lock = None;
        result
    }
}

#[cfg(test)]
#[path = "server_lifecycle_tests.rs"]
mod tests;
