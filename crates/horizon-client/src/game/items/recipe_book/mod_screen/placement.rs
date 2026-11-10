//! Placement state is discarded before a new catalogue or menu can be shown.
use super::*;
impl Game<'_> {
    pub(in crate::game) fn resolve_mod_book_declaration_slot(
        &self,
        slot: &mod_api::RecipeSlotDisplay,
    ) -> Vec<ItemStack> {
        let stocks = horizon_client::menu::place::Stacked::default();
        resolve::Resolver::new(self, &stocks).resolve(slot)
    }
    pub(in crate::game) fn install_mod_book_ghost(
        &mut self,
        selection: &horizon_client::recipe_book::placement_sync::Selection,
    ) {
        let Some(view) = self
            .native_net
            .placement
            .view
            .as_ref()
            .filter(|view| view.identity == selection.catalog)
        else {
            return;
        };
        let Some(layout) = view.index.layout(selection.catalog.menu.as_str()).cloned() else {
            return;
        };
        let Some(display) = view
            .index
            .categories()
            .flat_map(|category| &category.displays)
            .find(|entry| entry.recipe == selection.display.recipe.as_str())
            .and_then(|entry| entry.displays.get(selection.display.source as usize))
            .cloned()
        else {
            return;
        };
        let result = match layout.declaration() {
            mod_api::RecipeBookLayout::Crafting { result, .. }
            | mod_api::RecipeBookLayout::Furnace { result, .. }
            | mod_api::RecipeBookLayout::Stonecutter { result, .. }
            | mod_api::RecipeBookLayout::Smithing { result, .. } => *result,
        };
        let fuel_empty = match layout.declaration() {
            mod_api::RecipeBookLayout::Furnace { fuel, .. } => {
                self.slot_mut(300 + usize::from(*fuel)).is_none()
            }
            _ => true,
        };
        let Ok(mapped) = layout.map(&display, fuel_empty) else {
            return;
        };
        let mut ghosts = Vec::with_capacity(mapped.len());
        for (slot, display) in mapped {
            if self.native_screen_slot(usize::from(slot)).is_none() {
                return;
            }
            let items = self.resolve_mod_book_declaration_slot(&display);
            if !items.is_empty() {
                ghosts.push(super::super::Ghost {
                    slot: 300 + usize::from(slot),
                    items,
                    result: slot == result,
                });
            }
        }
        self.recipe_ui.ghost = ghosts;
    }
    pub(in crate::game) fn reset_mod_book_placement_ui(&mut self) {
        self.recipe_ui.ghost.clear();
        self.recipe_ui.custom.prepared = None;
        self.recipe_ui.custom.presentation = None;
        self.recipe_ui.custom.overlay = None;
    }
}
