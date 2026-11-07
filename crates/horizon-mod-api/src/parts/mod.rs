//! Parts (arismoko/horizon#41): code a mod attaches to one block, block entity, item or mod
//! menu type at startup. The scoped hooks ask the parts attached to the thing they are about,
//! after every mod's `Hooks` method. Parts attached by tag are resolved by each game against its
//! own tags, at world open and on `/reload`, into a table kept by that game's thread; that table
//! also indexes the block parts by state, so the state-keyed hooks look theirs up without the
//! block's name.
use crate::ticket_controller::{self, ControllerInfo};
use crate::{
    BlockEntityPart, BlockPart, BlockSide, BlockStateId, Capability, CapabilityKey,
    EntityCapabilityKey, EntityPart, EntityRef, HOOK_COUNT, HookId, Host, ItemCapabilityKey,
    ItemPart, ItemPlace, MenuPart, ModEntry, PlaceProvider, ServerHost, TicketController,
};
use horizon_mod_package::content::BlockEntityType;
use std::{
    any::{Any, type_name},
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    marker::PhantomData,
    num::NonZeroU32,
    rc::Rc,
    sync::{
        Arc, LazyLock, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

include!(concat!(env!("OUT_DIR"), "/parts_modules.rs"));

impl Registrar for PartRegistrar {
    fn block_entity_type_key(&mut self, id: &str) -> crate::BlockEntityTypeKey {
        match self
            .registry_keys
            .borrow_mut()
            .block_entity_types
            .request(self.mod_id, id)
        {
            Ok(slot) => crate::BlockEntityTypeKey(slot),
            Err(error) => {
                self.refused.push(error);
                crate::BlockEntityTypeKey(u32::MAX)
            }
        }
    }

    fn declare_block_entity_type(
        &mut self,
        path: &str,
        blocks: Vec<String>,
    ) -> crate::BlockEntityTypeKey {
        let id = format!("{}:{path}", self.mod_id);
        let key = self.block_entity_type_key(&id);
        self.block_entity_types.push(BlockEntityType { id, blocks });
        key
    }

    fn attach_block_entity_part(
        &mut self,
        key: &crate::BlockEntityTypeKey,
        kind: &'static str,
        part: Arc<dyn BlockEntityPart>,
    ) -> Result<(), crate::BlockEntityPartError> {
        let id = self
            .registry_keys
            .borrow()
            .block_entity_types
            .id(key.0)
            .map(str::to_owned)
            .unwrap_or_default();
        if id.starts_with("minecraft:") {
            return Err(crate::BlockEntityPartError::VanillaType { kind: *key });
        }
        if self.attach_known(|p| &mut p.block_entity_kinds, kind, &id, part) {
            Ok(())
        } else {
            Err(crate::BlockEntityPartError::Duplicate {
                kind: *key,
                part: kind,
            })
        }
    }

    fn provide_block_entity_capability(
        &mut self,
        name: &'static str,
        kind: &crate::BlockEntityTypeKey,
        before_game: bool,
        provider: BlockProviderOf,
    ) {
        self.capability(
            |p| &mut p.capabilities,
            name,
            ProviderTarget::BlockEntity(kind),
            before_game,
            {
                let handler = provider.handler();
                (provider, handler)
            },
        );
    }

    fn block_key(&mut self, id: &str) -> crate::BlockKey {
        match self
            .registry_keys
            .borrow_mut()
            .blocks
            .request(self.mod_id, id)
        {
            Ok(slot) => crate::BlockKey(slot),
            Err(error) => {
                self.refused.push(error);
                crate::BlockKey(u32::MAX)
            }
        }
    }

    fn attach_block_part(
        &mut self,
        block: crate::BlockKey,
        kind: &'static str,
        part: Arc<dyn BlockPart>,
    ) {
        let id = self
            .registry_keys
            .borrow()
            .blocks
            .id(block.0)
            .map(str::to_owned);
        let Some(id) = id else {
            self.refused.push(format!(
                "mod {}: invalid block key slot {}",
                self.mod_id, block.0
            ));
            return;
        };
        self.attach(|p| &mut p.blocks, kind, &id, part);
    }

    fn attach_block_part_tag(&mut self, tag: &str, kind: &'static str, part: Arc<dyn BlockPart>) {
        let part = self.tagged(kind, tag, part);
        self.tags.blocks.extend(part);
    }

    fn effect_key(&mut self, id: &str) -> crate::EffectKey {
        match self
            .registry_keys
            .borrow_mut()
            .effects
            .request(self.mod_id, id)
        {
            Ok(slot) => crate::EffectKey(slot),
            Err(error) => {
                self.refused.push(error);
                crate::EffectKey(u32::MAX)
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn attach_entity_part(&mut self, entity: &str, kind: &'static str, part: Arc<dyn EntityPart>) {
        self.attach(|p| &mut p.entities, kind, entity, part);
    }

    fn provide_entity_capability(
        &mut self,
        name: &'static str,
        kind: &str,
        before_game: bool,
        provider: EntityProviderOf,
    ) {
        self.capability(
            |p| &mut p.entity_capabilities,
            name,
            ProviderTarget::Id(kind),
            before_game,
            {
                let handler = provider.handler();
                (provider, handler)
            },
        );
    }

    fn fluid_key(&mut self, id: &str) -> crate::FluidKey {
        match self
            .registry_keys
            .borrow_mut()
            .fluids
            .request(self.mod_id, id)
        {
            Ok(slot) => crate::FluidKey(slot),
            Err(error) => {
                self.refused.push(error);
                crate::FluidKey(u32::MAX)
            }
        }
    }

    fn declare_fluid(&mut self, path: &str) -> crate::FluidKey {
        let id = format!("{}:{path}", self.mod_id);
        if path.contains(':') || !horizon_mod_package::content::namespaced_id(&id) {
            self.refused.push(format!(
                "mod {}: invalid fluid declaration {id:?}",
                self.mod_id
            ));
            return crate::FluidKey(u32::MAX);
        }
        let previous = self
            .registry_keys
            .borrow_mut()
            .declared_fluids
            .insert(id.clone(), self.mod_id);
        if let Some(other) = previous {
            self.refused.push(format!(
                "fluid {id} declared by both {other} and {}",
                self.mod_id
            ));
        }
        self.fluid_key(&id)
    }

    fn game_rule_category(&mut self, id: &str) -> Result<(), crate::GameRuleError> {
        let result = (|| {
            let id = self.game_rule_id(id)?;
            if self.game_rule_categories.contains(&id) {
                return Err(crate::GameRuleError::Duplicate);
            }
            self.game_rule_categories.push(id);
            Ok(())
        })();
        if let Err(error) = &result {
            self.refused
                .push(format!("mod {} category {id:?}: {error:?}", self.mod_id));
        }
        result
    }

    fn game_rule_bool(
        &mut self,
        id: &str,
        category: &str,
        default: bool,
    ) -> Result<crate::BoolGameRuleKey, crate::GameRuleError> {
        let result = (|| {
            let id = self.game_rule_id(id)?;
            if self.game_rules.iter().any(|r| r.name == id) {
                return Err(crate::GameRuleError::Duplicate);
            }
            let slot = self
                .registry_keys
                .borrow_mut()
                .game_rules
                .request(self.mod_id, &id)
                .map_err(|_| crate::GameRuleError::InvalidId)?;
            self.game_rules.push(horizon_client_core::gamerules::Rule {
                name: id,
                category: category.into(),
                default: horizon_client_core::gamerules::Value::Bool(default),
                min: 0,
                max: 0,
            });
            Ok(crate::BoolGameRuleKey(slot))
        })();
        if let Err(error) = &result {
            self.refused
                .push(format!("mod {} rule {id:?}: {error:?}", self.mod_id));
        }
        result
    }

    fn item_key(&mut self, id: &str) -> crate::ItemKey {
        match self
            .registry_keys
            .borrow_mut()
            .items
            .request(self.mod_id, id)
        {
            Ok(slot) => crate::ItemKey(slot),
            Err(error) => {
                self.refused.push(error);
                crate::ItemKey(u32::MAX)
            }
        }
    }

    fn attach_item_part(
        &mut self,
        item: crate::ItemKey,
        kind: &'static str,
        part: Arc<dyn ItemPart>,
    ) {
        let id = self
            .registry_keys
            .borrow()
            .items
            .id(item.0)
            .map(str::to_owned);
        let Some(id) = id else {
            self.refused.push(format!(
                "mod {}: invalid item key slot {}",
                self.mod_id, item.0
            ));
            return;
        };
        self.attach(|p| &mut p.items, kind, &id, part);
    }

    fn attach_item_part_tag(&mut self, tag: &str, kind: &'static str, part: Arc<dyn ItemPart>) {
        let part = self.tagged(kind, tag, part);
        self.tags.items.extend(part);
    }

    fn provide_item_capability(
        &mut self,
        name: &'static str,
        item: &str,
        before_game: bool,
        provider: ItemProviderOf,
    ) {
        self.capability(
            |p| &mut p.item_capabilities,
            name,
            ProviderTarget::Id(item),
            before_game,
            {
                let handler = provider.handler();
                (provider, handler)
            },
        );
    }

    /// Read and write the item places `ItemPlace::Mod { provider: name, .. }` with `provider`,
    /// as an accessory mod exposes its slots. One provider a name.
    fn place_provider(&mut self, name: &'static str, provider: Arc<dyn PlaceProvider>) {
        if !namespaced(name) {
            self.refused.push(format!(
                "{} provides item places {name:?}, which is not a namespaced name",
                self.mod_id
            ));
            return;
        }
        if let Some(other) = self.parts.place_providers.get(name) {
            self.refused.push(format!(
                "item places {name} are provided twice, by {} and {}",
                other.mod_id, self.mod_id
            ));
            return;
        }
        self.parts.place_providers.insert(
            name,
            PlaceProviderPart {
                mod_id: self.mod_id,
                mod_index: self.mod_index,
                provider,
            },
        );
    }

    fn register_tooltip_appender(
        &mut self,
        component: &str,
        placement: crate::TooltipPlacement,
        appender: Arc<dyn crate::TooltipAppender>,
    ) {
        let refusal = if !namespaced(component) {
            Some(format!(
                "{} registers a tooltip appender for {component:?}, which is not a namespaced data component id",
                self.mod_id
            ))
        } else {
            match &placement {
                crate::TooltipPlacement::Before(anchor)
                | crate::TooltipPlacement::After(anchor)
                    if !crate::TOOLTIP_DETAILS.contains(&anchor.as_str()) =>
                {
                    Some(format!(
                        "{} anchors a tooltip appender for {component:?} to {anchor:?}, which is not a component detail Horizon lists",
                        self.mod_id
                    ))
                }
                _ => None,
            }
        };
        if let Some(refusal) = refusal {
            self.refused.push(refusal);
            return;
        }
        self.parts.tooltip_appenders.push(TooltipAppenderPart {
            mod_id: self.mod_id,
            mod_index: self.mod_index,
            component: component.into(),
            placement,
            appender,
        });
    }

    fn register_key_mapping(&mut self, mapping: crate::NativeKeyRegistration) {
        if mapping.id.is_empty()
            || !mapping
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
            || self
                .parts
                .key_mappings
                .iter()
                .any(|(owner, prior)| *owner == self.mod_id && prior.id == mapping.id)
        {
            self.refused.push(format!(
                "mod {}: duplicate or invalid key mapping {:?}",
                self.mod_id, mapping.id
            ));
            return;
        }
        self.parts.key_mappings.push((self.mod_id, mapping));
    }

    fn attach_menu_part(&mut self, menu: &str, kind: &'static str, part: Arc<dyn MenuPart>) {
        self.attach(|p| &mut p.menus, kind, menu, part);
    }

    fn add_structure_type(&mut self, name: &str, ty: Arc<dyn crate::StructureType>) {
        let (namespace, path) = name.split_once(':').unwrap_or((self.mod_id, name));
        let id = format!("{}:{}", self.mod_id, path);
        let refusal = if namespace != self.mod_id {
            Some(format!(
                "{} registered structure type {name:?} in the namespace {namespace:?}, which is not its own",
                self.mod_id
            ))
        } else if path.is_empty() {
            Some(format!(
                "{} registered a structure type with no name",
                self.mod_id
            ))
        } else {
            self.structure_types
                .iter()
                .any(|t| *t.id == *id)
                .then(|| format!("{} registered structure type {id} twice", self.mod_id))
        };
        if let Some(refusal) = refusal {
            self.refused.push(refusal);
            return;
        }
        let engine_type = crate::structure_type::Adapter::new(&id, ty).into_engine_type();
        self.structure_types
            .push(crate::structure_type::StructureTypeInfo {
                id: id.into(),
                ty: engine_type,
            });
    }

    fn register_ticket_controller(&mut self, name: &str, validates: bool) -> TicketController {
        ticket_controller::register(
            &mut self.controllers,
            &mut self.refused,
            self.mod_id,
            name,
            validates,
        )
    }
    fn declare_entity_attributes(
        &mut self,
        path: &str,
        template: &str,
        attributes: Vec<(String, f64)>,
    ) -> Result<(), crate::EntityAttributeError> {
        let result = (|| {
            let (namespace, name) = path.split_once(':').unwrap_or((self.mod_id, path));
            if namespace.is_empty() || name.is_empty() || namespace != self.mod_id {
                return Err(crate::EntityAttributeError::InvalidId);
            }
            let (template_ns, template_name) =
                template.split_once(':').unwrap_or(("minecraft", template));
            if template_ns.is_empty() || template_name.is_empty() {
                return Err(crate::EntityAttributeError::InvalidId);
            }
            let id = format!("{namespace}:{name}");
            if self.entity_attributes.iter().any(|d| d.id == id) {
                return Err(crate::EntityAttributeError::Duplicate);
            }
            for (index, (attribute, base)) in attributes.iter().enumerate() {
                if !base.is_finite() {
                    return Err(crate::EntityAttributeError::NotFinite);
                }
                if attributes[..index]
                    .iter()
                    .any(|(other, _)| other == attribute)
                {
                    return Err(crate::EntityAttributeError::DuplicateAttribute);
                }
            }
            self.entity_attributes.push(EntityAttributeDeclaration {
                id,
                template: format!("{template_ns}:{template_name}"),
                attributes,
            });
            Ok(())
        })();
        if let Err(error) = &result {
            self.refused.push(format!(
                "mod {} entity attributes {path:?}: {error:?}",
                self.mod_id
            ));
        }
        result
    }
}

/// What a mod's [`Hooks::register`](crate::Hooks::register) attaches its parts with, and
/// declares its block entity types and ticket controllers on. A thing takes one part of each
/// kind: the same kind twice on one thing refuses the mods at startup, naming both.
///
/// Mods call the generic methods on `dyn Registrar` after requesting registry keys;
/// they reach the trait's own methods, one implementation for native and wasm mods.
pub trait Registrar {
    /// Look up a vanilla or another mod's type. The final registry validates it at load.
    fn block_entity_type_key(&mut self, id: &str) -> crate::BlockEntityTypeKey;
    /// Declare block entity type `path` in this mod's namespace, held by `blocks`; mods usually
    /// reach it through `block_entity_type`.
    fn declare_block_entity_type(
        &mut self,
        path: &str,
        blocks: Vec<String>,
    ) -> crate::BlockEntityTypeKey;
    /// Attach `part`, of kind `kind`, to the mod block entity type `key`.
    fn attach_block_entity_part(
        &mut self,
        key: &crate::BlockEntityTypeKey,
        kind: &'static str,
        part: Arc<dyn BlockEntityPart>,
    ) -> Result<(), crate::BlockEntityPartError>;
    /// Answer block entity capability `name` for type `kind` with `provider`, asked ahead of the
    /// game's own when `before_game`; mods reach it through `block_entity_capability`.
    fn provide_block_entity_capability(
        &mut self,
        name: &'static str,
        kind: &crate::BlockEntityTypeKey,
        before_game: bool,
        provider: BlockProviderOf,
    );

    /// Request a block by canonical registry id. All mods' declarations are checked at load.
    fn block_key(&mut self, id: &str) -> crate::BlockKey;
    /// Attach `part`, of kind `kind` (its type's name), to block `block`.
    fn attach_block_part(
        &mut self,
        block: crate::BlockKey,
        kind: &'static str,
        part: Arc<dyn BlockPart>,
    );
    /// Attach `part`, of kind `kind`, to every block in tag `tag`.
    fn attach_block_part_tag(&mut self, tag: &str, kind: &'static str, part: Arc<dyn BlockPart>);

    /// Request an effect by canonical registry id. All mods' declarations are checked at load.
    fn effect_key(&mut self, id: &str) -> crate::EffectKey;

    /// Attach `part`, of kind `kind`, to the entity type `entity`
    /// (`minecraft:zombie`, `mymod:golem`): its `register_goals` runs at every
    /// `Mob` construction of that type (#1077).
    ///
    /// Not on wasm: the goal surface a part's `register_goals` hands the game
    /// does not cross yet (#1077); entity parts are a native mod's for now.
    /// The gate keeps a wasm mod's call a compile error, not a runtime one;
    /// the generator still keeps the method off the wasm world by this doc.
    #[cfg(not(target_arch = "wasm32"))]
    fn attach_entity_part(&mut self, entity: &str, kind: &'static str, part: Arc<dyn EntityPart>);
    /// Answer entity capability `name` for entity type `kind` with `provider`; mods reach it
    /// through `entity_capability`.
    fn provide_entity_capability(
        &mut self,
        name: &'static str,
        kind: &str,
        before_game: bool,
        provider: EntityProviderOf,
    );

    /// Request a fluid by canonical registry id, checked when the registry loads.
    fn fluid_key(&mut self, id: &str) -> crate::FluidKey;
    /// Declare a tank-only fluid in this mod's namespace and return its key.
    fn declare_fluid(&mut self, path: &str) -> crate::FluidKey;

    /// Declare a game rule category in the calling mod's namespace. Invalid,
    /// foreign or duplicate IDs refuse mod loading even if the error is ignored.
    /// Categories appear in the host's game rules screen; remote clients do not
    /// receive them in this version.
    fn game_rule_category(&mut self, id: &str) -> Result<(), crate::GameRuleError>;
    /// Declare a boolean game rule in this mod's namespace. Category references are
    /// resolved after all mods register; an unknown category refuses installation then.
    fn game_rule_bool(
        &mut self,
        id: &str,
        category: &str,
        default: bool,
    ) -> Result<crate::BoolGameRuleKey, crate::GameRuleError>;

    /// Request an item by registry id, validated after all declarations.
    fn item_key(&mut self, id: &str) -> crate::ItemKey;
    /// Attach `part`, of kind `kind`, to item `item`.
    fn attach_item_part(
        &mut self,
        item: crate::ItemKey,
        kind: &'static str,
        part: Arc<dyn ItemPart>,
    );
    /// Attach `part`, of kind `kind`, to every item in tag `tag`.
    fn attach_item_part_tag(&mut self, tag: &str, kind: &'static str, part: Arc<dyn ItemPart>);
    /// Answer item capability `name` for item `item` with `provider`; mods reach it through
    /// `item_capability`.
    fn provide_item_capability(
        &mut self,
        name: &'static str,
        item: &str,
        before_game: bool,
        provider: ItemProviderOf,
    );
    /// Read and write the item places `ItemPlace::Mod { provider: name, .. }` with `provider`,
    /// as an accessory mod exposes its slots. One provider a name.
    fn place_provider(&mut self, name: &'static str, provider: Arc<dyn PlaceProvider>);
    /// Register `appender` as the lines data component `component` (`mymod:charge`) adds to a
    /// stack's tooltip, where `placement` puts them (NeoForge's `RegisterTooltipAppendersEvent`).
    /// Only this mod's appender is asked, for a stack that carries the component and whose
    /// tooltip shows it, and only while the mod is enabled. A component id that isn't namespaced,
    /// or a `Before`/`After` anchor the game doesn't list as a component detail, refuses the mods
    /// at startup, naming the mod.
    fn register_tooltip_appender(
        &mut self,
        component: &str,
        placement: crate::TooltipPlacement,
        appender: Arc<dyn crate::TooltipAppender>,
    );

    /// Register a Controls binding owned by the calling mod (`RegisterKeyMappingsEvent`).
    /// Called from `Hooks::register`; invalid or duplicate local ids refuse startup.
    fn register_key_mapping(&mut self, mapping: crate::NativeKeyRegistration);

    /// Attach `part`, of kind `kind`, to the mod menu type `menu`.
    fn attach_menu_part(&mut self, menu: &str, kind: &'static str, part: Arc<dyn MenuPart>);

    /// Register structure type `name` with `ty`; mods reach it through
    /// `register_structure_type`.
    fn add_structure_type(&mut self, name: &str, ty: Arc<dyn crate::StructureType>);

    /// Register ticket controller `name`, the way NeoForge's `RegisterTicketControllersEvent`
    /// does once at startup, and keep the handle for `chunk.set_mod_forced`. `name` is
    /// `quarry` or `mymod:quarry`; the controller's id is in the calling mod's namespace
    /// (`mymod:quarry`), and another mod's namespace refuses the mods at startup, as does an id
    /// registered twice. `validates` is whether it validates its saved tickets: the
    /// `validate_tickets` hook then runs for it when a level loads them (NeoForge's
    /// `LoadingValidationCallback`). A world's tickets of a controller no loaded mod registers are
    /// dropped when it loads.
    fn register_ticket_controller(&mut self, name: &str, validates: bool) -> TicketController;
    /// Declare the initial attribute set of the mod's entity type `path`
    /// (NeoForge's `EntityAttributeCreationEvent`), starting from `template`'s
    /// (`minecraft:zombie`, the one the type's content file copies, as
    /// `Zombie.createAttributes` builds on the template's): each attribute's
    /// id (`minecraft:max_health`) and the base value that differs from the
    /// template's. The values reach the type's own fields, its live attribute
    /// map and the attribute sync, so the reads of #1469 and #1493 answer the
    /// same defaults with no second copy.
    ///
    /// Resolved once, at load: an id outside the mod's namespace, a duplicate
    /// attribute, a non-finite base or a second declaration of the same type
    /// refuses the mods, naming the mod; an attribute the game does not know,
    /// or a template without attribute defaults, refuses the resource load,
    /// naming the type.
    fn declare_entity_attributes(
        &mut self,
        path: &str,
        template: &str,
        attributes: Vec<(String, f64)>,
    ) -> Result<(), crate::EntityAttributeError>;
}

impl CapabilityProviders for InstalledProviders {
    fn block_entity_provider(
        &self,
        name: &str,
        kind: crate::BlockEntityTypeKey,
        index: &mut u32,
    ) -> Option<(BlockProviderOf, &'static str, bool)> {
        let kind = crate::registry_keys::block_entity_type_name(kind)?;
        next(&installed()?.keys.capabilities, name, kind, index)
    }

    fn provided(&self, by: ProvidedBy, name: &str) -> bool {
        installed().is_some_and(|installed| match by {
            ProvidedBy::BlockEntity => installed.keys.capabilities.contains_key(name),
            ProvidedBy::Item => installed.keys.item_capabilities.contains_key(name),
            ProvidedBy::Entity => installed.keys.entity_capabilities.contains_key(name),
        })
    }

    fn entity_provider(
        &self,
        name: &str,
        kind: &str,
        index: &mut u32,
    ) -> Option<(EntityProviderOf, &'static str, bool)> {
        next(&installed()?.keys.entity_capabilities, name, kind, index)
    }

    fn item_provider(
        &self,
        name: &str,
        item: &str,
        index: &mut u32,
    ) -> Option<(ItemProviderOf, &'static str, bool)> {
        next(&installed()?.keys.item_capabilities, name, item, index)
    }
    fn place_provider(&self, name: &str) -> Option<(Arc<dyn PlaceProvider>, &'static str)> {
        let part = installed()?.keys.place_providers.get(name)?;
        crate::mod_index_on(part.mod_index).then(|| (part.provider.clone(), part.mod_id))
    }
}

/// How a lookup reads the capability providers mods registered, in the order they are asked
/// (those registered `before_game`, then the others, each in registration order): the game's
/// table, which a wasm mod reads through the game.
// hidden: a lookup's view of the registrar's table, not part of the mod surface
#[doc(hidden)]
pub trait CapabilityProviders {
    /// The next enabled provider of block entity capability `name` for block entity type `kind`,
    /// with its mod and whether it is asked ahead of the game. `index` starts at zero and advances
    /// past every visited registration, including disabled ones.
    fn block_entity_provider(
        &self,
        name: &str,
        kind: crate::BlockEntityTypeKey,
        index: &mut u32,
    ) -> Option<(BlockProviderOf, &'static str, bool)>;

    /// Whether any mod registered capability `name` for the things `by` names. This presence
    /// check includes disabled mods; enumeration skips their providers.
    fn provided(&self, by: ProvidedBy, name: &str) -> bool;

    /// The next enabled provider of entity capability `name` for entity type `kind`, advancing `index`.
    fn entity_provider(
        &self,
        name: &str,
        kind: &str,
        index: &mut u32,
    ) -> Option<(EntityProviderOf, &'static str, bool)>;

    /// The next enabled provider of item capability `name` for item `item`, advancing `index`.
    fn item_provider(
        &self,
        name: &str,
        item: &str,
        index: &mut u32,
    ) -> Option<(ItemProviderOf, &'static str, bool)>;
    /// The enabled provider of the item places named `name`, with its mod.
    fn place_provider(&self, name: &str) -> Option<(Arc<dyn PlaceProvider>, &'static str)>;
}
