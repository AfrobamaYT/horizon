//! Load-owned declarations and resolved mappings for registration-slot keys.
use crate::{BlockKey, BlockStateId, BlockStateRange, EffectKey};
use std::collections::HashMap;

#[derive(Clone, Default)]
pub(crate) struct Requests {
    pub(crate) blocks: Names,
    pub(crate) effects: Names,
}

#[derive(Clone, Default)]
pub(crate) struct Names {
    entries: Vec<Request>,
    slots: HashMap<String, u32>,
}

#[derive(Clone)]
struct Request {
    id: String,
    mods: Vec<&'static str>,
}

impl Names {
    pub(crate) fn request(&mut self, mod_id: &'static str, id: &str) -> Result<u32, String> {
        // Match the registry boundary's identifier classification, including the default namespace.
        let id = if id.contains(':') {
            id.to_owned()
        } else {
            format!("minecraft:{id}")
        };
        if !horizon_mods::content::namespaced_id(&id) {
            return Err(format!("mod {mod_id}: invalid registry key {id:?}"));
        }
        let slot = *self.slots.entry(id.clone()).or_insert_with(|| {
            let slot = u32::try_from(self.entries.len()).expect("registry key slots exhausted");
            self.entries.push(Request {
                id,
                mods: Vec::new(),
            });
            slot
        });
        let mods = &mut self.entries[slot as usize].mods;
        if !mods.contains(&mod_id) {
            mods.push(mod_id);
        }
        Ok(slot)
    }

    pub(crate) fn id(&self, slot: u32) -> Option<&str> {
        self.entries
            .get(slot as usize)
            .map(|entry| entry.id.as_str())
    }

    fn resolve<T>(
        &self,
        kind: &str,
        mut resolve: impl FnMut(&str) -> Option<T>,
    ) -> Result<Vec<T>, Vec<String>> {
        let mut values = Vec::with_capacity(self.entries.len());
        let mut errors = Vec::new();
        for entry in &self.entries {
            match resolve(&entry.id) {
                Some(value) => values.push(value),
                None => {
                    errors.extend(entry.mods.iter().map(|mod_id| {
                        format!("mod {mod_id} requested unknown {kind} {}", entry.id)
                    }))
                }
            }
        }
        if errors.is_empty() {
            Ok(values)
        } else {
            Err(errors)
        }
    }
}

/// A block slot's mappings in one game's current tables.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedBlock {
    /// The block's default state.
    pub default: BlockStateId,
    /// Its consecutive state ids.
    pub states: BlockStateRange,
    /// Its index in the behaviour table.
    pub block: horizon_world::level::states::BlockId,
}

thread_local! {
    static BLOCKS: std::cell::RefCell<Vec<ResolvedBlock>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Rebind this game's slots at the owning load/reload boundary. A refused reload clears the
/// mapping, so no operation can accidentally use states from the previous tables.
pub fn resolve_blocks(
    mut resolve: impl FnMut(&str) -> Option<ResolvedBlock>,
) -> Result<(), Vec<String>> {
    let result = crate::part_table::requested_keys()
        .blocks
        .resolve("block", &mut resolve);
    BLOCKS.with(|blocks| {
        *blocks.borrow_mut() = Vec::new();
        match result {
            Ok(values) => {
                *blocks.borrow_mut() = values;
                Ok(())
            }
            Err(errors) => Err(errors),
        }
    })
}

/// The current numeric mapping; also refuses an out-of-range slot at the owning table boundary.
pub fn block(key: BlockKey) -> Option<ResolvedBlock> {
    BLOCKS.with(|blocks| blocks.borrow().get(key.0 as usize).copied())
}

/// The resolved effect slots. Effects remain fixed for the process, as does their registry.
#[derive(Default)]
pub(crate) struct Effects {
    pub(crate) names: Names,
    pub(crate) indices: Vec<usize>,
}

/// Resolve requested effects once, after every mod's content has been registered.
pub fn resolve_effects() -> Result<(), Vec<String>> {
    use horizon_client_core::potions;
    let installed = crate::part_table::requested_keys();
    let all: Vec<_> = potions::all_effects().collect();
    let mut names = installed.effects.clone();
    // Hooks and effect enumeration may expose entries no mod explicitly requested.
    for def in &all {
        names
            .request("horizon", potions::namespaced(def.id))
            .expect("registry owns valid ids");
    }
    let indices = names.resolve("effect", |id| {
        all.iter().position(|def| potions::namespaced(def.id) == id)
    })?;
    let _ = crate::part_table::resolved_effects().set(Effects { names, indices });
    Ok(())
}

/// Resolve an effect operation by numeric slot, never by a fresh name scan.
pub fn effect(key: EffectKey) -> Option<&'static horizon_client_core::potions::EffectDef> {
    let index = *crate::part_table::resolved_effects()
        .get()?
        .indices
        .get(key.0 as usize)?;
    let vanilla = horizon_client_core::potions::EFFECTS;
    if index < vanilla.len() {
        vanilla.get(index)
    } else {
        horizon_client_core::potions::mod_effects().get(index - vanilla.len())
    }
}

/// Canonical diagnostic name, or refusal for an invalid slot.
pub fn effect_id(key: EffectKey) -> Option<&'static str> {
    crate::part_table::resolved_effects().get()?.names.id(key.0)
}

/// Canonical diagnostic name, or refusal for an invalid slot.
pub fn block_id(key: BlockKey) -> Option<&'static str> {
    crate::part_table::requested_keys().blocks.id(key.0)
}
