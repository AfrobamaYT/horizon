use super::*;

/// Helpers for native mods, kept as stable as the hook points.
// Native mods use these; nothing in Horizon does.
#[allow(dead_code)]
impl Game<'_> {
    /// Presses button `button` of the open menu, as a click on one of its
    /// buttons does (`MultiPlayerGameMode.handleInventoryButtonClick`,
    /// `ServerboundContainerButtonClickPacket`): the server's menu decides
    /// what it does (`AbstractContainerMenu.clickMenuButton`, and
    /// `native_menu_button` there for a mod's menu). For a client mod's own
    /// input on a menu's screen where `native_screen_click` doesn't come — a
    /// key, the wheel. No sound: that is the screen's. Err while no menu the
    /// server opened is open.
    pub(in crate::game) fn native_click_menu_button(&self, button: i32) -> Result<(), String> {
        let Some(container) = self
            .remote
            .as_ref()
            .and_then(|r| r.menu.as_ref())
            .map(|(w, _)| *w)
        else {
            return Err("no menu is open".to_owned());
        };
        self.send(
            horizon_protocol::packets::play::Serverbound::ContainerButtonClick(
                horizon_protocol::packets::play::ContainerButtonClick { container, button },
            ),
        );
        Ok(())
    }
}
