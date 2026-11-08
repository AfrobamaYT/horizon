//! The `entity_entering_section` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `entity` moved from the 16x16x16 section `old_section` into `new_section` (NeoForge's
    /// `EntityEvent.EnteringSection`, vanilla's `EntitySectionStorage` callbacks on
    /// `Entity.setPosRaw`). Both are section coordinates (`SectionPos`: each block coordinate
    /// shifted right by 4), and `chunk_changed` is NeoForge's `didChunkChange`: the x or z
    /// section differs, so the entity is in another chunk column. `host` is the side that saw
    /// the move. Nothing can be cancelled: the entity is already in its new section.
    ///
    /// Told on each side for every entity of the level it shows: on the server for each
    /// entity of every kind in each level and for each player in it, on a client for its own
    /// player and for each entity its server told it of, where the server last placed it.
    /// Walking, being pushed, riding, a teleport and a client's position packet all count
    /// alike. Each side looks once a tick (the server at the end of each level's tick, before
    /// `level_tick_post`; a client after its tick) and tells each entity once whose section
    /// changed since it last looked, from the section it was in then: two crossings by one
    /// entity within a tick are told as one, and a move out and back within a tick is not
    /// told. An entity joining a level, changing level, or a player respawning starts in its
    /// section untold, as vanilla adds it to its section without this event.
    #[hook(
        side = "both",
        contract = "listen",
        neoforge = "EntityEvent.EnteringSection",
        cancellable = false
    )]
    fn entity_entering_section();
}
