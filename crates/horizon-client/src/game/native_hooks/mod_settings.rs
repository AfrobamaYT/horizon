use super::*;

/// The typed reads of the settings a mod registered (`Registrar::register_setting`), for the
/// mod whose hook or part is running (`crash::current_native_mod`): a mod reads only its own.
// Native mods use these; nothing in Horizon does.
#[allow(dead_code)]
impl Game<'_> {
    /// The calling mod's setting `key` as kept in its data folder, or its default.
    fn own_setting(&self, key: &str) -> Option<mod_api::SettingValue> {
        let mod_id = mod_api::crash::current_native_mod()?;
        let spec = mod_api::__private::setting(mod_id, key)?;
        let kept = self.setting(mod_id, &spec.key);
        Some(spec.read(kept.as_deref()))
    }
    /// The calling mod's toggle `key`: the player's choice, else its default.
    pub(in crate::game) fn setting_bool(&self, key: &str) -> Option<bool> {
        match self.own_setting(key)? {
            mod_api::SettingValue::Toggle(on) => Some(on),
            _ => None,
        }
    }
    /// The option the player picked for the calling mod's choice `key`, else its default.
    pub(in crate::game) fn setting_choice(&self, key: &str) -> Option<u32> {
        match self.own_setting(key)? {
            mod_api::SettingValue::Choice(index) => u32::try_from(index).ok(),
            _ => None,
        }
    }
    /// The calling mod's range `key`, or its default.
    pub(in crate::game) fn setting_f64(&self, key: &str) -> Option<f64> {
        match self.own_setting(key)? {
            mod_api::SettingValue::Range(value) => Some(value),
            _ => None,
        }
    }
    /// The calling mod's colour `key`, `0xRRGGBB`, or its default.
    pub(in crate::game) fn setting_color(&self, key: &str) -> Option<u32> {
        match self.own_setting(key)? {
            mod_api::SettingValue::Color(rgb) => Some(rgb),
            _ => None,
        }
    }
}
