//! The keys mods register (`Registrar::register_key_mapping`) as plain `KeyMapping`s: held while
//! their binding is down and counting its presses for `consume_click`, from the window's keys and
//! mouse buttons and the control socket's alike. The control socket's never fire `input_key`.
use super::Game;
use super::key_codes::{gameplay_has_keyboard, glfw_modifiers};
use horizon_client::keymapping::KeyAction;
use mod_api::{KeyConflictContext, KeyRegistration};
use winit::{event::MouseButton, keyboard::KeyCode};

impl Game<'_> {
    /// `KeyMapping.set` and `KeyMapping.click` for the mods' mappings bound to the physical `key`
    /// (`KeyboardHandler.keyPress`).
    pub(super) fn mod_mappings_key(&mut self, key: KeyCode, action: KeyAction) {
        if let Some(name) = horizon_client::keybinds::key_name(key) {
            self.mod_mappings_input(name, action, false);
        }
    }
    /// The same for a mouse button (`MouseHandler.onButton`).
    pub(super) fn mod_mappings_button(&mut self, button: MouseButton, pressed: bool) {
        let action = if pressed {
            KeyAction::Press
        } else {
            KeyAction::Release
        };
        self.mod_mappings_input(&horizon_client::keybinds::mouse_name(button), action, true);
    }
    /// The control socket's `key KEY down|up|tap|repeat`: what the window's key would do to the
    /// mappings.
    pub(super) fn agent_mod_mappings_key(&mut self, key: KeyCode, action: &str) {
        if action != "up" {
            let press = if action == "repeat" {
                KeyAction::Repeat
            } else {
                KeyAction::Press
            };
            self.mod_mappings_key(key, press);
        }
        if matches!(action, "up" | "tap") {
            self.mod_mappings_key(key, KeyAction::Release);
        }
    }
    /// `input` is a key's or a mouse button's name (`key.keyboard.z`, `key.mouse.left`). A press
    /// sets and clicks each mapping bound to it whose conflict context and modifiers are active
    /// (NeoForge's `isActiveAndMatches`); a release lets go of every one bound to it.
    fn mod_mappings_input(&mut self, input: &str, action: KeyAction, mouse: bool) {
        let mappings = self.mod_mappings();
        if mappings.is_empty() {
            return;
        }
        let screen_open = !gameplay_has_keyboard(
            self.screen,
            self.title.is_some(),
            self.screen_over_playing(),
        );
        let held = glfw_modifiers(self.modifiers);
        for (index, (owner, mapping)) in mappings.iter().enumerate() {
            let (key, modifiers) = self.mod_binding(owner, mapping);
            if key != input
                || action != KeyAction::Release
                    && !active(mapping.context, modifiers, held, screen_open)
            {
                continue;
            }
            let state = self.native_input.mappings.get_mut(index);
            if mouse {
                state.button(action != KeyAction::Release, screen_open, false);
            } else {
                state.key(action, screen_open, false);
            }
        }
    }
    /// The mods' registered mappings, in registration order: none off the client.
    fn mod_mappings(&self) -> &'static [(&'static str, KeyRegistration)] {
        super::mods::collect_key_mappings(
            self.options.execution_role,
            mod_api::__private::key_mappings(),
        )
    }
    /// A mapping's saved key and modifiers, or its declared default.
    fn mod_binding(&self, owner: &str, mapping: &KeyRegistration) -> (String, u32) {
        self.settings
            .mods
            .binding(owner, &mapping.id, (&mapping.key, mapping.modifiers.bits))
    }
    /// Mod `mod_id`'s mapping `id` and its place in the registration list.
    fn mod_mapping(&self, mod_id: &str, id: &str) -> Option<(usize, &'static KeyRegistration)> {
        self.mod_mappings()
            .iter()
            .enumerate()
            .find(|(_, (owner, mapping))| *owner == mod_id && mapping.id == id)
            .map(|(index, (_, mapping))| (index, mapping))
    }
    /// `KeyMapping.isDown`: held, while its conflict context and modifiers are active.
    pub(in crate::game) fn is_down(&self, mod_id: &str, id: &str) -> bool {
        let Some((index, mapping)) = self.mod_mapping(mod_id, id) else {
            return false;
        };
        let screen_open = !gameplay_has_keyboard(
            self.screen,
            self.title.is_some(),
            self.screen_over_playing(),
        );
        let (_, modifiers) = self.mod_binding(mod_id, mapping);
        self.native_input.mappings.is_down(index)
            && active(
                mapping.context,
                modifiers,
                glfw_modifiers(self.modifiers),
                screen_open,
            )
    }
    /// `KeyMapping.consumeClick`.
    pub(in crate::game) fn consume_click(&mut self, mod_id: &str, id: &str) -> bool {
        self.mod_mapping(mod_id, id)
            .is_some_and(|(index, _)| self.native_input.mappings.consume_click(index))
    }
}

/// `isConflictContextAndModifierActive`: the context's screens are the ones showing (in game with
/// none open, a GUI with one) and every modifier the binding holds is down (`KeyModifier.isActive`).
fn active(context: KeyConflictContext, modifiers: u32, held: u32, screen_open: bool) -> bool {
    let shown = match context {
        KeyConflictContext::InGame => !screen_open,
        KeyConflictContext::Gui => screen_open,
        _ => true,
    };
    shown && modifiers & held == modifiers
}
