//! Container geometry constructed by the screen build seam.
use super::{Creative, Game, Screen};
use super::screen_build::ContainerLayout;
use horizon_client::inventory_effects::EffectsLayout;

impl Game<'_> {
    /// The open container screen's layout from the game's state: the panel it
    /// draws, the book and effect panels beside it, the creative tab rows and
    /// every slot's hover box. The build calls it when it makes the layout;
    /// nothing else does.
    pub(super) fn container_layout(
        &self,
        batch: &horizon_client::hud::HudBatch,
    ) -> ContainerLayout {
        let image = self.container_image_parts(batch);
        let origin = self.inventory_origin_unbuilt(batch);
        // A title over the inventory draws no book or effects of its own.
        let covered = self.screen == Screen::Inventory && self.title.is_some();
        let book = (!covered).then(|| self.recipe_book_rect(batch)).flatten();
        let shows_effects = !covered
            && match self.screen {
                Screen::Creative => true,
                Screen::Inventory => {
                    self.container.is_none() && self.craft_size == 2 && !self.mount_active()
                }
                _ => false,
            };
        let space = Self::effects_space(batch, [origin.0, origin.1], image[2]);
        let effects_layout = if shows_effects {
            self.inventory_effects_layout(&space, image)
        } else {
            EffectsLayout::vanilla(space.available())
        };
        let effects = shows_effects
            .then(|| {
                self.inventory_effects_rect(batch, [origin.0, origin.1], image[2], &effects_layout)
            })
            .flatten();
        let tabs = (self.screen == Screen::Creative).then(|| {
            [
                [
                    image[0],
                    image[1] - Creative::TAB_ROW_DEPTH,
                    image[2],
                    Creative::TAB_ROW_DEPTH,
                ],
                [
                    image[0],
                    image[1] + image[3],
                    image[2],
                    Creative::TAB_ROW_DEPTH,
                ],
            ]
        });
        let slots = self
            .slots()
            .into_iter()
            .map(|slot| {
                let (x, y) = self.slot_position(slot);
                (slot, [origin.0 + x - 1.0, origin.1 + y - 1.0, 18.0, 18.0])
            })
            .collect();
        ContainerLayout {
            image,
            origin,
            book,
            effects,
            effects_layout,
            tabs,
            slots,
        }
    }

}
