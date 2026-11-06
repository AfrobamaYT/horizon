/// What answers a capability for a block entity of one type: given the block entity and the
/// side asked about, its handler, or none to leave it to the next provider. A closure
/// `|host, at| Some(Box::new(Tank(at)))` is one.
pub trait BlockCapabilityProvider<T: ?Sized>: Send + Sync {
    /// The handler for the block entity at `at`, or none.
    fn provide(&self, host: &mut dyn ServerHost, at: BlockSide) -> Option<Box<T>>;
}
impl<T: ?Sized, F> BlockCapabilityProvider<T> for F
where
    F: Fn(&mut dyn ServerHost, BlockSide) -> Option<Box<T>> + Send + Sync,
{
    fn provide(&self, host: &mut dyn ServerHost, at: BlockSide) -> Option<Box<T>> {
        self(host, at)
    }
}
/// A block capability provider, as the registrar keeps it.
pub type BlockProvider<T> = dyn BlockCapabilityProvider<T>;

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
        kind: super::BlockEntityTypeKey,
        index: &mut u32,
    ) -> Option<(BlockProviderOf, &'static str, bool)>;
}

impl CapabilityProviders for InstalledProviders {
    fn block_entity_provider(
        &self,
        name: &str,
        kind: super::BlockEntityTypeKey,
        index: &mut u32,
    ) -> Option<(BlockProviderOf, &'static str, bool)> {
        let kind = crate::registry_keys::block_entity_type_name(kind)?;
        next(&installed()?.keys.capabilities, name, kind, index)
    }
}
pub use providers_of::{BlockProviderOf, EntityProviderOf, ItemProviderOf};

/// The block entity types mods registered.
pub fn block_entity_types() -> &'static [BlockEntityType] {
    installed().map_or(&[], |installed| &installed.block_entity_types)
}

pub(crate) fn resolved_block_entity_types() -> &'static OnceLock<super::registry_keys::Names> {
    static EMPTY: OnceLock<super::registry_keys::Names> = OnceLock::new();
    installed().map_or(&EMPTY, |installed| &installed.resolved_block_entity_types)
}

impl HookParts {
    pub(crate) fn block_entity<'k>(
        &self,
        id: impl FnOnce() -> Option<&'k str>,
    ) -> &[Attached<dyn BlockEntityPart>] {
        attached(&self.block_entities, id)
    }
    pub(crate) fn block_entity_kind<'k>(
        &self,
        id: impl FnOnce() -> Option<&'k str>,
    ) -> &[Attached<dyn BlockEntityPart>] {
        attached(&self.block_entity_kinds, id)
    }
}

impl Parts {
    #[cfg(test)]
    pub(crate) fn block_entity<'k>(
        &self,
        id: impl FnOnce() -> Option<&'k str>,
    ) -> &[Attached<dyn BlockEntityPart>] {
        attached(&self.block_entities, id)
    }
}

/// The providers mods registered for capability `key` and the type of the block entity at
/// `at`, for the game's own lookups (a hopper's): none where no mod registered one, so a
/// vanilla container's lookup costs one table check and allocates nothing, and the game needn't
/// work out its own answer. [`BlockEntityProviders::ask`] then asks them.
pub fn block_entity_providers<T: ?Sized + 'static>(
    key: &CapabilityKey<T>,
    host: &dyn Host,
    at: BlockSide,
) -> Option<BlockEntityProviders<T>> {
    let parts = installed()?.keys.capabilities.get(key.name)?.get(
        super::registry_keys::block_entity_type_name(at.kind(host)?)?,
    )?;
    Some(BlockEntityProviders {
        parts,
        at,
        handler: PhantomData,
    })
}

/// A block entity's providers of one capability ([`block_entity_providers`]).
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct BlockEntityProviders<T: ?Sized + 'static> {
    parts: &'static [CapabilityPart<BlockProviderOf>],
    at: BlockSide,
    handler: PhantomData<fn() -> Box<T>>,
}

impl<T: ?Sized + 'static> BlockEntityProviders<T> {
    /// A mod's handler where the game would answer with its own unless a mod comes first: one
    /// registered `before_game`, else, where the game has no answer (`game`), the first mod's.
    pub fn ask(self, host: &mut dyn ServerHost, game: bool) -> Option<Capability<T>> {
        let at = self.at;
        let answer = ask_providers(
            host,
            |index| next_provider(self.parts, index),
            BlockProviderOf::get::<T>,
            |provider, host| provider.provide(host, at),
            |_| game,
        );
        match answer {
            Answer::Mod(found) => Some(found),
            Answer::Game | Answer::Nobody => None,
        }
    }
}

/// What a mod's [`Hooks::register`](super::Hooks::register) attaches its parts with, and
/// declares its block entity types and ticket controllers on. A thing takes one part of each
/// kind: the same kind twice on one thing refuses the mods at startup, naming both.
///
/// Mods call the generic methods on `dyn Registrar` after requesting registry keys;
/// they reach the trait's own methods, one implementation for native and wasm mods.
pub trait Registrar {
    /// Look up a vanilla or another mod's type. The final registry validates it at load.
    fn block_entity_type_key(&mut self, id: &str) -> super::BlockEntityTypeKey;
    /// Declare block entity type `path` in this mod's namespace, held by `blocks`; mods usually
    /// reach it through `block_entity_type`.
    fn declare_block_entity_type(
        &mut self,
        path: &str,
        blocks: Vec<String>,
    ) -> super::BlockEntityTypeKey;
    /// Attach `part`, of kind `kind`, to the mod block entity type `key`.
    fn attach_block_entity_part(
        &mut self,
        key: &super::BlockEntityTypeKey,
        kind: &'static str,
        part: Arc<dyn BlockEntityPart>,
    ) -> Result<(), super::BlockEntityPartError>;
    /// Answer block entity capability `name` for type `kind` with `provider`, asked ahead of the
    /// game's own when `before_game`; mods reach it through `block_entity_capability`.
    fn provide_block_entity_capability(
        &mut self,
        name: &'static str,
        kind: &super::BlockEntityTypeKey,
        before_game: bool,
        provider: BlockProviderOf,
    );
}

impl dyn Registrar + '_ {
    /// Declare a block entity type in this mod's namespace.
    pub fn block_entity_type(&mut self, id: &str) -> super::BlockEntityTypeBuilder<'_> {
        super::BlockEntityTypeBuilder {
            id: id.to_owned(),
            registrar: self,
            blocks: Vec::new(),
        }
    }
    /// Attach a Part to a native type, including one another mod declares later.
    /// Vanilla keys are usable for capabilities, but not for native payload Parts.
    pub fn block_entity_part<P: BlockEntityPart>(
        &mut self,
        kind: &super::BlockEntityTypeKey,
        part: P,
    ) -> Result<(), super::BlockEntityPartError> {
        self.attach_block_entity_part(kind, type_name::<P>(), Arc::new(part))
    }
    /// Answer capability `key` for block entity type `kind` with `provider`, which gives
    /// a handler for one block entity and side, or none:
    /// `r.block_entity_capability(&ITEMS, &press, |_, _| Some(Box::new(PressItems)))`.
    /// Like NeoForge's `RegisterCapabilitiesEvent`, providers add up: in the fork's chosen order
    /// (NeoForge's javadoc states none), the game's own is asked first, then the mods' in
    /// registration order, and the first to answer wins.
    pub fn block_entity_capability<T: ?Sized + 'static>(
        &mut self,
        key: &CapabilityKey<T>,
        kind: &super::BlockEntityTypeKey,
        provider: impl Fn(&mut dyn ServerHost, BlockSide) -> Option<Box<T>> + Send + Sync + 'static,
    ) {
        self.provide_block_entity_capability(
            key.name,
            kind,
            false,
            BlockProviderOf::of(Arc::new(provider)),
        );
    }
    /// `block_entity_capability`, asked ahead of the game's own
    /// provider (NeoForge's priority), as a mod that replaces how a block entity's slots are
    /// reached.
    pub fn block_entity_capability_before_game<T: ?Sized + 'static>(
        &mut self,
        key: &CapabilityKey<T>,
        kind: &super::BlockEntityTypeKey,
        provider: impl Fn(&mut dyn ServerHost, BlockSide) -> Option<Box<T>> + Send + Sync + 'static,
    ) {
        self.provide_block_entity_capability(
            key.name,
            kind,
            true,
            BlockProviderOf::of(Arc::new(provider)),
        );
    }
}

impl PartRegistrar {
    fn expand_block_entity_parts(&mut self) {
        for (id, parts) in &self.parts.block_entity_kinds {
            let Some(kind) = self.block_entity_types.iter().find(|kind| kind.id == **id) else {
                for part in parts {
                    self.refused.push(format!(
                        "mod {} attached {} to undeclared native block entity type {id}",
                        part.mod_id, part.type_name
                    ));
                }
                continue;
            };
            for block in &kind.blocks {
                for part in parts {
                    let block_part: Arc<dyn BlockPart> = part.part.clone();
                    if insert_part(
                        &mut self.parts.blocks,
                        block,
                        part.with_part(block_part),
                        &mut self.refused,
                    ) {
                        insert_part(
                            &mut self.parts.block_entities,
                            block,
                            part.clone(),
                            &mut self.refused,
                        );
                    }
                }
            }
        }
        for parts in self.parts.blocks.values_mut() {
            parts.sort_by_key(|part| (part.mod_index, part.order));
        }
        for parts in self.parts.block_entities.values_mut() {
            parts.sort_by_key(|part| (part.mod_index, part.order));
        }
    }
}

impl Registrar for PartRegistrar {
    fn block_entity_type_key(&mut self, id: &str) -> super::BlockEntityTypeKey {
        match self
            .registry_keys
            .borrow_mut()
            .block_entity_types
            .request(self.mod_id, id)
        {
            Ok(slot) => super::BlockEntityTypeKey(slot),
            Err(error) => {
                self.refused.push(error);
                super::BlockEntityTypeKey(u32::MAX)
            }
        }
    }

    fn declare_block_entity_type(
        &mut self,
        path: &str,
        blocks: Vec<String>,
    ) -> super::BlockEntityTypeKey {
        let id = format!("{}:{path}", self.mod_id);
        let key = self.block_entity_type_key(&id);
        self.block_entity_types.push(BlockEntityType { id, blocks });
        key
    }

    fn attach_block_entity_part(
        &mut self,
        key: &super::BlockEntityTypeKey,
        kind: &'static str,
        part: Arc<dyn BlockEntityPart>,
    ) -> Result<(), super::BlockEntityPartError> {
        let id = self
            .registry_keys
            .borrow()
            .block_entity_types
            .id(key.0)
            .map(str::to_owned)
            .unwrap_or_default();
        if id.starts_with("minecraft:") {
            return Err(super::BlockEntityPartError::VanillaType { kind: *key });
        }
        if self.attach_known(|p| &mut p.block_entity_kinds, kind, &id, part) {
            Ok(())
        } else {
            Err(super::BlockEntityPartError::Duplicate {
                kind: *key,
                part: kind,
            })
        }
    }

    fn provide_block_entity_capability(
        &mut self,
        name: &'static str,
        kind: &super::BlockEntityTypeKey,
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
}
/// Fixtures that tests of other crates use to install Parts.
#[cfg(any(test, feature = "testing"))]
pub mod tests {

    #[test]
    fn block_entity_slots_resolve_forward_declarations_and_refuse_unknown() {
        struct Request(Arc<std::sync::Mutex<Vec<BlockEntityTypeKey>>>);
        #[mod_api::__subscriptions]
        impl Hooks for Request {
            fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
                let later = r.block_entity_type_key("owner:press");
                assert_eq!(later, r.block_entity_type_key("owner:press"));
                assert_eq!(BlockEntityTypeKey::from_slot(later.0), Ok(later));
                assert!(BlockEntityTypeKey::from_slot(later.0 + 1).is_err());
                self.0.lock().unwrap().push(later);
            }
        }
        struct Declare(Arc<std::sync::Mutex<Vec<BlockEntityTypeKey>>>);
        #[mod_api::__subscriptions]
        impl Hooks for Declare {
            fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
                let declared = r.block_entity_type("press").build();
                let chest = r.block_entity_type_key("chest");
                self.0.lock().unwrap().extend([declared, chest]);
            }
        }
        let keys = Arc::new(std::sync::Mutex::new(Vec::new()));
        install_parts(&[
            ModEntry::new("first", Box::new(Request(keys.clone()))),
            ModEntry::new("owner", Box::new(Declare(keys.clone()))),
        ])
        .unwrap();
        let types = |names: &[&str]| {
            horizon_client_core::block_entity_types::BlockEntityTypes::new(
                Default::default(),
                names.iter().map(|s| (*s).to_owned()).collect(),
            )
        };
        assert_eq!(
            crate::registry_keys::resolve_block_entity_types(&types(&["minecraft:chest"]))
                .unwrap_err(),
            [
                "mod first requested unknown block entity type owner:press",
                "mod owner requested unknown block entity type owner:press"
            ]
        );
        crate::registry_keys::resolve_block_entity_types(&types(&[
            "minecraft:chest",
            "owner:press",
            "minecraft:furnace",
        ]))
        .unwrap();
        let keys = keys.lock().unwrap();
        assert_eq!(keys[0], keys[1]);
        assert_ne!(keys[0], keys[2]);
        assert_eq!(
            crate::registry_keys::block_entity_type_key("owner:press"),
            Some(keys[0])
        );
        assert_eq!(
            crate::registry_keys::block_entity_type_name(keys[2]),
            Some("minecraft:chest")
        );
        let furnace = crate::registry_keys::block_entity_type_key("minecraft:furnace").unwrap();
        assert_ne!(furnace, keys[2]);
        assert_eq!(BlockEntityTypeKey::from_slot(furnace.0), Ok(furnace));
        assert!(BlockEntityTypeKey::from_slot(u32::MAX).is_err());
        struct Table;
        impl Backend for Table {
            fn block_entity_type_key_id(&self, key: BlockEntityTypeKey) -> String {
                crate::registry_keys::block_entity_type_name(key)
                    .unwrap()
                    .to_owned()
            }
        }
        impl Host for Table {
            fn backend(&self) -> Option<BackendRef<'_>> {
                Some(BackendRef::of(self))
            }
        }
        assert_eq!(keys[0].id(&Table), "owner:press");
    }

    #[test]
    fn invalid_block_entity_type_paths_refuse_registration() {
        struct Invalid;
        #[mod_api::__subscriptions]
        impl super::super::Hooks for Invalid {
            fn register(&self, r: &mut dyn Registrar, _: &crate::RegistrationContext) {
                for path in ["", "other:press", "Uppercase", "has space"] {
                    r.block_entity_type(path).build();
                }
            }
        }
        let errors = install_parts(&[ModEntry::new("test", Box::new(Invalid))]).unwrap_err();
        assert_eq!(errors.len(), 4);
        assert!(
            errors
                .iter()
                .all(|error| error.starts_with("mod test: invalid registry key"))
        );
    }

    /// The block entity of type `doubler:press_entity`, held by block `test:press`: it ticks, and
    /// each tick adds a data slot.
    struct Press;
    #[mod_api::horizon_part]
    impl BlockPart for Press {
        fn analog_output(&self, _: &dyn LevelHost, event: &mut AnalogOutputEvent) {
            event.output += 40;
        }
    }
    #[mod_api::horizon_part]
    impl BlockEntityPart for Press {
        fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
            event.ticks = true;
        }
        fn block_entity_tick(&self, _: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
            event.entity.data.push(1);
        }
        fn block_entity_view_distance(
            &self,
            _: &dyn ClientHost,
            event: &mut BlockEntityViewDistanceEvent<'_>,
        ) {
            event.distance = 128.0;
        }
    }
    /// The gold a `doubler:hoard` block entity holds, kept beside it.
    pub const HOARD: AttachmentType<u32> = AttachmentType {
        namespace: "test",
        name: "hoard",
        default: || 0,
        codec: None,
        copy_on_death: false,
        sync: false,
    };
    /// How many times a `doubler:hoard` was asked about its removal.
    pub static HOARD_ASKED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    /// The block entity of type `doubler:hoard`, held by block `test:hoard`: when it is
    /// removed it gives back the gold its `HOARD` counts.
    struct Hoard(BlockKey);
    #[mod_api::horizon_part]
    impl BlockPart for Hoard {}
    #[mod_api::horizon_part]
    impl BlockEntityPart for Hoard {
        /// With data `[1]`, it replaces its own block with granite.
        fn block_entity_tick(
            &self,
            host: &mut dyn ServerHost,
            event: &mut BlockEntityTickEvent<'_>,
        ) {
            if event.entity.data == [1] {
                let level = host.level("minecraft:overworld").unwrap();
                let state = host.block_state_default(self.0).unwrap();
                assert!(level.set_block(
                    host,
                    event.pos,
                    state,
                    NativeBlockUpdateFlags::KNOWN_SHAPE
                ));
            }
        }
        fn pre_remove_side_effects(
            &self,
            level: &mut dyn LevelRandomHost,
            event: &mut PreRemoveSideEffectsEvent<'_>,
        ) {
            HOARD_ASKED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let BlockPos { x, y, z } = event.pos;
            if let Ok(Some(count @ 1..)) = HOARD.get_existing(level, Holder::BlockEntity([x, y, z]))
            {
                event
                    .drops
                    .push(ItemStack::new("minecraft:gold_ingot", count));
            }
        }
    }

    #[test]
    fn a_vanilla_type_refuses_a_native_payload_part_at_registration() {
        let mut r = super::PartRegistrar::new("test");
        let kind = r.block_entity_type_key("minecraft:chest");
        assert_eq!(
            (&mut r as &mut dyn Registrar).block_entity_part(&kind, Press),
            Err(BlockEntityPartError::VanillaType { kind }),
        );
        assert!(r.parts.block_entity_kinds.is_empty());
        assert!(r.parts.blocks.is_empty());
        assert!(r.parts.block_entities.is_empty());
    }

    #[test]
    fn a_duplicate_native_part_returns_its_typed_error_and_keeps_the_first() {
        let mut r = super::PartRegistrar::new("test");
        let kind = (&mut r as &mut dyn Registrar)
            .block_entity_type("press")
            .blocks(&["test:press"])
            .build();
        (&mut r as &mut dyn Registrar)
            .block_entity_part(&kind, Press)
            .unwrap();
        assert_eq!(
            (&mut r as &mut dyn Registrar).block_entity_part(&kind, Press),
            Err(BlockEntityPartError::Duplicate {
                kind,
                part: type_name::<Press>()
            }),
        );
        assert_eq!(r.parts.block_entity_kinds["test:press"].len(), 1);
    }

    #[test]
    fn a_block_entity_part_answers_by_block_and_by_entity_type() {
        game_thread();
        let ticks = |block| {
            let mut ticks = false;
            dispatch_block_entity_ticks(&NoHost, block, &mut ticks);
            ticks
        };
        // The ticker is asked by block: the type id is not a block.
        assert!(ticks("test:press"));
        assert!(!ticks("doubler:press_entity"));
        let tick = |kind: &str| {
            let mut entity = ModBlockEntity::new(kind.into(), Default::default());
            dispatch_block_entity_tick(&mut NoHost, [0, 64, 0], &mut entity);
            entity.data.len()
        };
        // The tick is asked by the entity's type, which differs from its block's id.
        assert_eq!(tick("doubler:press_entity"), 1);
        assert_eq!(tick("test:press"), 0);
        // The view distance is asked by block, as the ticker is.
        let distance = |block| {
            let mut distance = 64.0;
            dispatch_block_entity_view_distance(&NoHost, block, &mut distance);
            distance
        };
        assert_eq!(distance("test:press"), 128.0);
        assert_eq!(distance("doubler:press_entity"), 64.0);
        assert_eq!(distance("test:other"), 64.0);
    }
}
