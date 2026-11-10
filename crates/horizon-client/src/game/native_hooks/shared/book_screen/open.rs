use super::*;

impl Game<'_> {
    /// The container slots (of `size`) that the mod's screen for the block at
    /// `pos` does not show: the server's menu does not have them either, so a
    /// shift-click never moves an item where the player cannot see it.
    pub(in crate::game) fn native_hidden_slots(&self, pos: Pos, size: usize) -> Vec<usize> {
        if self.mod_menu_at(pos).is_none() {
            return Vec::new();
        }
        let block = self
            .res
            .facts()
            .get(self.world.block(pos.0, pos.1, pos.2))
            .name
            .to_string();
        let mut screen = None;
        self.native_screen(&block, &mut screen);
        let Some(screen) = screen else {
            return Vec::new();
        };
        (0..size)
            .filter(|&i| screen.slots.get(i).is_none_or(Option::is_none))
            .collect()
    }
    /// A menu opened on this client with its slots under `key`, on block
    /// `target`: a mod block's is laid out by its mod (`native_screen`).
    /// `extra` is what the server sent for this window first.
    pub(in crate::game) fn open_native_screen(
        &mut self,
        key: Pos,
        target: Option<Pos>,
        window: i32,
        menu: &str,
        extra: Option<horizon_protocol::menu_extra::Message>,
    ) {
        self.mod_screen = None;
        let Some(t) = target else {
            return;
        };
        let block = self
            .res
            .facts()
            .get(self.world.block(t.0, t.1, t.2))
            .name
            .to_string();
        if block.starts_with("minecraft:") {
            return;
        }
        // Only when this is the menu the clicked block opens: `target` outlives the
        // use that set it, so another menu opened later (a command's, another
        // mod's) would otherwise get the last clicked machine's screen. Asked
        // here, as the window's world has no mod containers.
        let mut container = None;
        self.native_block_entity_container(&block, &mut container);
        let opens = container.and_then(|c| mod_menu(c.menu.as_deref()?).map(|(m, _)| m));
        if opens != Some(menu) {
            return;
        }
        let mut screen = None;
        self.native_screen(&block, &mut screen);
        if let Some(screen) = screen {
            if let Err(error) = validate_book_geometry(menu, &screen) {
                log::error!("mod menu {menu} screen refused: {error}");
                return;
            }
            self.mod_screen = Some(OpenNativeScreen {
                layout_revision: {
                    static NEXT: std::sync::atomic::AtomicU64 =
                        std::sync::atomic::AtomicU64::new(1);
                    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                },
                key,
                window,
                block,
                screen,
                data: Vec::new(),
                extra,
            });
        }
    }
}

#[derive(Debug)]
enum ScreenBookError {
    WrongBook,
    UnknownMenu,
    InvalidLayout(horizon_client::recipe_book::slot_layout::LayoutError),
}

fn validate_book_geometry(menu: &str, screen: &mod_api::Screen) -> Result<(), ScreenBookError> {
    use horizon_client::recipe_book::{
        menu_layout::{MenuLayoutMetadata, compile},
        slot_layout::LayoutError,
    };
    let Some((book, _, layout)) = mod_api::RecipeBookLayout::registered(menu) else {
        return Ok(());
    };
    if screen.recipe_book.as_ref() != Some(&book) {
        return Err(ScreenBookError::WrongBook);
    }
    let menu =
        horizon_client::containers::mod_menu_type(menu).ok_or(ScreenBookError::UnknownMenu)?;
    let metadata = MenuLayoutMetadata {
        slots: u16::try_from(menu.slots)
            .map_err(|_| ScreenBookError::InvalidLayout(LayoutError::InvalidSlotCount))?,
        crafting: menu
            .crafting
            .map(|grid| (grid.grid as u16, grid.size as u16, grid.result as u16)),
    };
    let layout = compile(layout, metadata).map_err(ScreenBookError::InvalidLayout)?;
    layout
        .validate_geometry(&screen.slots)
        .map_err(ScreenBookError::InvalidLayout)
}

impl std::fmt::Display for ScreenBookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongBook => f.write_str("screen book differs from menu declaration"),
            Self::UnknownMenu => f.write_str("content menu is unavailable"),
            Self::InvalidLayout(error) => write!(f, "{error:?}"),
        }
    }
}
