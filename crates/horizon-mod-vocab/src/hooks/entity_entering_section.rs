//! The `entity_entering_section` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `entity` moved from the 16x16x16 section `old_section` into `new_section` (NeoForge's
    /// `EntityEvent.EnteringSection`, vanilla's `EntitySectionStorage` callbacks on
    /// `Entity.setPosRaw`). Both are section coordinates (`SectionPos`: each block coordinate
    /// shifted right by 4), and `chunk_changed` is NeoForge's `didChunkChange`: the x or z
    /// section differs, so the entity is in another chunk column. `host` is the side that saw
    /// the move. Nothing can be cancelled: the entity is already in its new section.
    ///
    /// Told on each side as the move happens, once for every section change, from the
    /// section the entity was in before that move: two moves told apart are two events, and a
    /// move out and one back are two. The moves: an entity's own move in its tick (on the
    /// server every kind in the level's lists, the fishing hooks and the dragon; on a client
    /// each entity it shows, where its interpolation puts it), each move a server player sent,
    /// a teleport (`/tp`, a mod's `teleport_to`), a piston or shulker push, and on a client
    /// each packet that places an entity (its own player's teleport, a vehicle it drives). Moves
    /// within one entity's tick are one move. An entity joining a level, changing level, or a
    /// player respawning starts in its section untold, as vanilla adds it to its section
    /// without this event; one that moves before its first tick is told. What a listener moves
    /// is told at the next move, not from inside its own call.
    #[hook(
        side = "both",
        contract = "listen",
        neoforge = "EntityEvent.EnteringSection",
        cancellable = false
    )]
    fn entity_entering_section();
}
