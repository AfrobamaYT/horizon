use super::*;

impl Game<'_> {
    /// Effective HUD and first-person hand visibility.
    pub(in crate::game) fn hud_mode(&self) -> mod_api::HudMode {
        self.ui.hud_mode()
    }

    /// Set HUD and hand visibility outside a borrowed server guest's turn.
    pub(in crate::game) fn set_hud_mode(&mut self, mode: mod_api::HudMode) {
        if self.window() && self.lan_acting.is_none() {
            self.ui.set_hud_mode(mode);
        }
    }
}
