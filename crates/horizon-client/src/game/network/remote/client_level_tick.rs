use super::*;

/// The client's per-level tick, `ClientLevel.tick`: the window's work for the
/// level in place, bracketed by `ClientTickEvents.START_LEVEL_TICK` and
/// `END_LEVEL_TICK`. A client with no level, or with no listener for either
/// hook, pays one `listened` check a tick; a tick that fails returns before
/// the end hook, as Fabric's injection at `ClientLevel.tick`'s RETURN does not
/// run for an exception.
impl Game<'_> {
    pub(in crate::game) fn tick_remote_level(&mut self) -> Result<()> {
        // `ClientTickEvents.START_LEVEL_TICK`/`END_LEVEL_TICK`: the level is
        // built and both hooks dispatched only for a level in place and a
        // listener of either, so a client without a level or a mod pays one
        // `listened` check a tick.
        let level = (mod_api::__private::listened(mod_api::HookId::ClientLevelTickPre)
            || mod_api::__private::listened(mod_api::HookId::ClientLevelTickPost))
        .then(|| self.native_client_level())
        .flatten();
        if let Some(level) = level
            && mod_api::__private::listened(mod_api::HookId::ClientLevelTickPre)
        {
            self.native_client_level_tick_pre(level);
        }
        self.ticks += 1;
        self.repro_tick();
        // Hud.tick ends with ChatComponent.tick.
        let chat_ticks = self.visual_ticks();
        self.ui.chat.tick(&self.gui.atlas, chat_ticks);
        self.tick_sign_editor();
        self.track_experience_display();
        self.tick_enchanting_book();
        // Leaving a slot clears its bundle's selection here and on the server.
        self.tick_bundle_hover();
        // Options.broadcastOptions: a changed render distance or Chat option
        // is sent again.
        let info = self
            .settings
            .client_information(client_view_distance(self.settings.render_distance));
        if let Some(r) = self.remote.as_mut()
            && r.joined
            && r.sent_info != info
        {
            r.sent_info = info.clone();
            self.send(S::ClientInformation(info));
        }
        // Minecraft.tick: TickRateManager.tick; ClientLevel.tickTime advances
        // the time only while the level runs normally.
        let running = self.clock.world_runs();
        self.boss_overlay.tick_darken(running);
        // Under `HORIZON_STILL` the frame's countdowns (this one, the hotbar's pop, the HUD's
        // below) wait for the world, as the border's size change does in any frozen level: a
        // frozen client keeps ticking, as many ticks as the run took.
        let hold = !running && self.clock.still();
        if !hold {
            self.tick_item_activation();
        }
        if running {
            self.world.game_time += 1;
            self.remote_clock_tick();
        }
        // Weather levels drift toward their targets between server updates.
        let w = &mut self.world.weather;
        w.rain_level.0 = w.rain_level.1;
        w.thunder_level.0 = w.thunder_level.1;
        if self.use_delay > 0 {
            self.use_delay -= 1;
        }
        // Minecraft.handleKeybinds precedes entity ticking and sendChanges.
        // Window callbacks only queue clicks, so an interaction cannot leak
        // between CLIENT_TICK_END and a later pong.
        self.remote_handle_input();
        let result = self.tick_remote_player(running, hold);
        // `ClientLevel.tick`'s RETURN, which a failed tick does not reach.
        if result.is_ok()
            && let Some(level) = level
            && mod_api::__private::listened(mod_api::HookId::ClientLevelTickPost)
        {
            self.native_client_level_tick_post(level);
        }
        result
    }
}
