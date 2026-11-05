//! Structure template lookup.

pub trait Helpers {
    /// `native_structure_template(id, at, rotation, mirror)` reads structure template `id` (`StructureTemplateManager.get`: the world's saved structures, then the game's and the mods' own) and answers what `/place template` would place at `at`: its size and its blocks, each with its position, its turned state and its block entity data, in vanilla's placement order. Nothing is placed: a builder places the blocks a tick at a time with `native_set_block`. Bundles need "hooks": @API:structure_template@.
    #[helper(
        side = "server",
        target = "horizon_client::game::native_structures::Game<'_>::native_structure_template"
    )]
    fn structure_template();
}
