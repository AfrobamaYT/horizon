//! The game-owned conversion boundary between API records and engine values.
//! Rust's orphan rule prevents foreign From/TryFrom implementations here.
use horizon_core::nbt::{JavaString, Tag};
use horizon_items::items::{ItemStack as EngineStack, Items};
use mod_api::item_stack::{ItemStack as ApiStack, StackWriteError};

/// Convert an engine value into an owned API value after resolving registry identities.
pub(in crate::game) trait TryIntoApi {
    type Api;
    type Error;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error>;
}

/// Validate a mod value against the game's registry before admitting it to engine state.
pub(in crate::game) trait TryIntoEngine {
    type Engine;
    type Error;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::game) enum StackReadError {
    UnknownItem,
    Empty,
}

impl TryIntoApi for EngineStack {
    type Api = ApiStack;
    type Error = StackReadError;

    fn try_into_api(&self) -> Result<ApiStack, Self::Error> {
        let key = mod_api::__private::item_key(&self.item).ok_or(StackReadError::UnknownItem)?;
        let mut result = ApiStack::new(key, self.count).ok_or(StackReadError::Empty)?;
        result.set_damage(self.damage);
        if let Some(Tag::Compound(mut patch)) = horizon_items::item_data::encode_components(self) {
            patch.remove(&JavaString::from("minecraft:damage"));
            result.set_components(patch);
        }
        Ok(result)
    }
}

impl TryIntoEngine for ApiStack {
    type Engine = EngineStack;
    type Error = StackWriteError;

    fn try_into_engine(&self, items: &Items) -> Result<EngineStack, Self::Error> {
        let id =
            mod_api::__private::item_key_id(self.item()).ok_or(StackWriteError::UnknownItem)?;
        let (id, definition) = items
            .items
            .get_key_value(id)
            .ok_or(StackWriteError::UnknownItem)?;
        let patch = self
            .components()
            .map(|patch| Tag::Compound(patch.as_nbt().clone()));
        let (_, extra) = horizon_items::item_data::decode_components(patch.as_ref());
        let stack = EngineStack {
            item: id.clone(),
            count: self.count(),
            damage: self.damage(),
            extra,
        };
        let effective = |name: &str, default: u32| -> Result<u32, StackWriteError> {
            match stack.data().and_then(|data| data.other(name)) {
                Some(Tag::Int(value)) => {
                    u32::try_from(*value).map_err(|_| StackWriteError::InvalidMaximum)
                }
                Some(_) => Err(StackWriteError::InvalidMaximum),
                None => Ok(default),
            }
        };
        let maximum = effective("minecraft:max_stack_size", definition.max_stack)?;
        if stack.count > maximum {
            return Err(StackWriteError::CountExceeded { maximum });
        }
        let maximum = effective("minecraft:max_damage", definition.max_damage)?;
        if stack.damage > maximum {
            return Err(StackWriteError::DamageExceeded { maximum });
        }
        Ok(stack)
    }
}
