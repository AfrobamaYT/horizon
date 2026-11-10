//! A mod menu's book uses the shared painter, but never vanilla storage ordinals.
use super::*;
use horizon_client::recipe_book::mod_presentation::ModPresentation;
mod resolve;

#[derive(Default)]
pub(super) struct ModRecipeUi {
    binding: Option<(String, String)>,
    selected: Option<String>,
    page: usize,
    presentation: Option<ModPresentation>,
    overlay: Option<OverlayView>,
    stocks: Vec<ItemStack>,
    query: String,
    filtering: bool,
}

impl Game<'_> {
    fn mod_book_panel(&self) -> Option<horizon_client::recipe_book::ModBookPanel<'_>> {
        let key = self.native_screen_book()?;
        let menu = self.native_screen_menu()?;
        self.recipe_tables
            .books
            .as_ref()?
            .as_ref()
            .ok()?
            .panel(menu, key)
    }
    pub(super) fn mod_book_flags(&self) -> Option<(bool, bool)> {
        self.mod_book_panel()?;
        let flags = self.custom_book_settings(self.native_screen_book()?)?;
        Some((flags.open, flags.filtering))
    }
    pub(super) fn mod_book_open(&self) -> bool {
        self.mod_book_flags().is_some_and(|flags| flags.0)
    }
    fn refresh_mod_book(&mut self) {
        self.ensure_mod_book_displays();
        let Some((open, filtering)) = self.mod_book_flags() else {
            return;
        };
        let Some(key) = self.native_screen_book() else {
            return;
        };
        let binding = (
            self.native_screen_menu().unwrap_or_default().to_owned(),
            key.as_str().to_owned(),
        );
        if self.recipe_ui.custom.binding.as_ref() != Some(&binding) {
            self.recipe_ui.reset_screen();
            self.recipe_ui.custom.binding = Some(binding);
        }
        self.ensure_recipe_search();
        if !open {
            self.recipe_ui.custom.overlay = None;
            if let Some(search) = self.recipe_ui.search.as_mut() {
                search.focused = false;
            }
            return;
        }
        let query = self
            .recipe_ui
            .search
            .as_ref()
            .map_or("", |s| s.value.as_str())
            .to_owned();
        let mut stocks: Vec<_> = self.inventory.iter().flatten().cloned().collect();
        for slot in self.native_screen_slots().unwrap_or_default() {
            if let Some(stack) = self.slot_mut(slot).clone() {
                stocks.push(stack);
            }
        }
        let ui = &self.recipe_ui.custom;
        if ui.presentation.is_some()
            && ui.query == query
            && ui.filtering == filtering
            && ui.stocks == stocks
        {
            return;
        }
        let resolver = resolve::Resolver::new(self, &stocks);
        let Some(panel) = self.mod_book_panel() else {
            return;
        };
        let presentation = panel.presentation(ui.selected.as_deref(), &query, filtering, &resolver);
        let ui = &mut self.recipe_ui.custom;
        ui.query = query;
        ui.filtering = filtering;
        ui.stocks = stocks;
        ui.page = ui
            .page
            .min(presentation.recipes.len().div_ceil(20).saturating_sub(1));
        ui.overlay = None;
        ui.presentation = Some(presentation);
    }
    fn mod_book_button(&self, batch: &HudBatch) -> (f32, f32) {
        let (ox, oy) = self.inventory_origin(batch);
        (
            ox + self.native_screen_width().unwrap_or(176.0) + 4.0,
            oy + 5.0,
        )
    }
    pub(super) fn draw_mod_book(&mut self, batch: &mut HudBatch) {
        self.refresh_mod_book();
        let Some((open, filtering)) = self.mod_book_flags() else {
            return;
        };
        let (bx, by) = self.mod_book_button(batch);
        batch.sprite(
            &self.gui.atlas,
            &sprite(if inside(self.cursor, bx, by, 20.0, 18.0) {
                "button_highlighted"
            } else {
                "button"
            }),
            bx,
            by,
        );
        if !open {
            return;
        }
        if !self.shift_down_or_ctrl() {
            self.recipe_ui.time += self.frame_ticks;
        }
        let (xo, yo) = self.book_origin(batch);
        let search = match self.recipe_ui.search.as_mut() {
            Some(edit) => {
                edit.x = xo + 25.0;
                edit.y = yo + 13.0;
                let (display, blink_ms) = edit.view_state(&self.gui.atlas);
                Search {
                    value: edit.value.clone(),
                    cursor: edit.cursor,
                    highlight: edit.highlight,
                    display,
                    focused: edit.focused,
                    blink_ms,
                }
            }
            None => Search::default(),
        };
        let ui = &self.recipe_ui.custom;
        let Some(presentation) = &ui.presentation else {
            return;
        };
        let buttons: Vec<_> = presentation
            .recipes
            .iter()
            .skip(ui.page * 20)
            .take(20)
            .map(|r| r.button.clone())
            .collect();
        let translate = |key: &str| self.translate_with_fallback(key, key);
        let text_width = |s: &str| HudBatch::text_width(&self.gui.atlas, s) as i32;
        let layout = Layout {
            book: BookType::Crafting,
            width: batch.width as i32,
            height: batch.height as i32,
            narrow: batch.width < NARROW,
            mouse: (self.cursor.0 as i32, self.cursor.1 as i32),
            index: self.swap_index(),
            filtering,
            search: &search,
            tabs: &presentation.tabs,
            buttons: &buttons,
            page: ui.page,
            pages: presentation.recipes.len().div_ceil(20),
            overlay: ui.overlay.as_ref(),
            ghost: None,
            translate: &translate,
            text_width: &text_width,
        };
        let calls = screen::draw(&layout);
        if let Some(lines) = self.paint_book(batch, &calls) {
            self.draw_tooltip_lines(batch, &lines, self.cursor.0, self.cursor.1);
        }
    }
    pub(super) fn mod_book_click(&mut self, _right: bool) -> bool {
        self.refresh_mod_book();
        let Some((open, filtering)) = self.mod_book_flags() else {
            return false;
        };
        let Some(batch) = self.batch_for_size() else {
            return false;
        };
        let (bx, by) = self.mod_book_button(&batch);
        let cursor = self.cursor;
        if inside(cursor, bx, by, 20.0, 18.0) {
            if let Some(book) = self.native_screen_book().cloned() {
                self.request_custom_book_settings(
                    &book,
                    mod_api::RecipeBookSettings {
                        open: !open,
                        filtering,
                    },
                );
            }
            return true;
        }
        if !open {
            return false;
        }
        let narrow = batch.width < NARROW;
        if self.spectator() {
            return narrow;
        }
        let (xo, yo) = self.book_origin(&batch);
        if self.recipe_ui.custom.overlay.take().is_some() {
            return true;
        }
        if inside(cursor, xo + 110.0, yo + 12.0, 26.0, 16.0) {
            if let Some(book) = self.native_screen_book().cloned() {
                self.request_custom_book_settings(
                    &book,
                    mod_api::RecipeBookSettings {
                        open,
                        filtering: !filtering,
                    },
                );
            }
            return true;
        }
        if let Some(search) = self.recipe_ui.search.as_mut() {
            search.x = xo + 25.0;
            search.y = yo + 13.0;
            if inside(cursor, xo + 8.0, yo + 13.0, 98.0, 14.0) {
                self.recipe_ui.search_drag = true;
                search.focused = true;
                search.reset_blink();
                if cursor.0 >= search.x {
                    let index = search.index_at(&self.gui.atlas, cursor.0);
                    search.set_cursor(index, false);
                }
                return true;
            }
            search.focused = false;
        }
        let ui = &mut self.recipe_ui.custom;
        let Some(presentation) = &ui.presentation else {
            return narrow;
        };
        let pages = presentation.recipes.len().div_ceil(20);
        if ui.page + 1 < pages && inside(cursor, xo + 93.0, yo + 137.0, 12.0, 17.0) {
            ui.page += 1;
            return true;
        }
        if ui.page > 0 && inside(cursor, xo + 38.0, yo + 137.0, 12.0, 17.0) {
            ui.page -= 1;
            return true;
        }
        for (n, id) in presentation.tab_ids.iter().enumerate() {
            if inside(cursor, xo - 30.0, yo + 3.0 + 27.0 * n as f32, 35.0, 27.0) {
                ui.selected = (n != 0).then(|| id.clone());
                ui.page = 0;
                ui.presentation = None;
                return true;
            }
        }
        for (n, recipe) in presentation
            .recipes
            .iter()
            .skip(ui.page * 20)
            .take(20)
            .enumerate()
        {
            let (x, y) = Self::button_pos(xo, yo, n);
            if inside(cursor, x, y, 25.0, 25.0) {
                // Presentation only: neither click can place a mod recipe through
                // vanilla RecipeDisplayId packets or manufacture ghost slots.
                let (x, y) = screen::overlay_origin(
                    x as i32,
                    y as i32,
                    recipe.overlay.len(),
                    xo as i32,
                    yo as i32,
                );
                ui.overlay = Some(OverlayView {
                    x,
                    y,
                    recipes: recipe.overlay.clone(),
                });
                return true;
            }
        }
        narrow
    }
    pub(super) fn mod_book_area(&self, batch: &HudBatch) -> Option<(bool, bool)> {
        let (open, _) = self.mod_book_flags()?;
        let (xo, yo) = self.book_origin(batch);
        let ui = &self.recipe_ui.custom;
        let hovered = ui.presentation.as_ref().is_some_and(|p| {
            p.tabs.iter().enumerate().any(|(n, tab)| {
                tab.selected
                    && inside(
                        self.cursor,
                        xo - 30.0,
                        yo + 3.0 + 27.0 * n as f32,
                        35.0,
                        27.0,
                    )
            })
        });
        Some((open, open && hovered))
    }
    pub(super) fn mod_book_targets(&self) -> Vec<horizon_client::widgets::Target> {
        use horizon_client::widgets::{Kind, Target};
        let (Some((open, _)), Some(batch)) = (self.mod_book_flags(), self.batch_for_size()) else {
            return Vec::new();
        };
        let button = |rect| Target {
            rect,
            kind: Kind::Button,
        };
        let (bx, by) = self.mod_book_button(&batch);
        let mut targets = vec![button([bx, by, 20.0, 18.0])];
        if !open || self.spectator() {
            return targets;
        }
        let (xo, yo) = self.book_origin(&batch);
        let ui = &self.recipe_ui.custom;
        if let Some(overlay) = &ui.overlay {
            let columns = if overlay.recipes.len() <= 16 { 4 } else { 5 };
            return (0..overlay.recipes.len())
                .map(|n| {
                    button([
                        overlay.x as f32 + 4.0 + 25.0 * (n % columns) as f32,
                        overlay.y as f32 + 5.0 + 25.0 * (n / columns) as f32,
                        24.0,
                        24.0,
                    ])
                })
                .collect();
        }
        targets.push(Target {
            rect: [xo + 25.0, yo + 13.0, 81.0, 14.0],
            kind: Kind::Field,
        });
        targets.push(button([xo + 110.0, yo + 12.0, 26.0, 16.0]));
        let Some(p) = &ui.presentation else {
            return targets;
        };
        for (n, tab) in p.tabs.iter().enumerate() {
            targets.push(Target {
                rect: [xo - 30.0, yo + 3.0 + 27.0 * n as f32, 35.0, 27.0],
                kind: Kind::Tab(tab.selected),
            });
        }
        for n in 0..p.recipes.len().saturating_sub(ui.page * 20).min(20) {
            let (x, y) = Self::button_pos(xo, yo, n);
            targets.push(button([x, y, 25.0, 25.0]));
        }
        if ui.page > 0 {
            targets.push(button([xo + 38.0, yo + 137.0, 12.0, 17.0]));
        }
        if ui.page + 1 < p.recipes.len().div_ceil(20) {
            targets.push(button([xo + 93.0, yo + 137.0, 12.0, 17.0]));
        }
        targets
    }
}
