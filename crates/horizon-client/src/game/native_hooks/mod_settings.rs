use super::*;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// The typed values read so far, by the mod's data folder and the key: a read resolves the
/// setting and its file once, and a write (`set_setting`) forgets the key.
static READ: LazyLock<Mutex<HashMap<(PathBuf, String), mod_api::SettingValue>>> =
    LazyLock::new(Default::default);

/// The typed reads of the settings a mod registered (`Registrar::register_setting`), for the
/// mod whose hook or part is running (`crash::current_native_mod`): a mod reads only its own.
// Native mods use these; nothing in Horizon does.
#[allow(dead_code)]
impl Game<'_> {
    /// The calling mod's setting `key` as kept in its data folder, or its default.
    fn own_setting(&self, key: &str) -> Option<mod_api::SettingValue> {
        let mod_id = mod_api::crash::current_native_mod()?;
        let at = (self.native_data_dir(mod_id), key.to_owned());
        if let Some(value) = READ.lock().ok()?.get(&at) {
            return Some(value.clone());
        }
        let spec = mod_api::__private::setting(mod_id, key)?;
        let value = spec.read(self.setting(mod_id, &spec.key).as_deref());
        READ.lock().ok()?.insert(at, value.clone());
        Some(value)
    }
    /// Drops what [`READ`] keeps of setting `key` in data folder `dir`.
    pub(in crate::game) fn forget_setting(&self, dir: &Path, key: &str) {
        if let Ok(mut read) = READ.lock() {
            read.remove(&(dir.to_path_buf(), key.to_owned()));
        }
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
