//! Structure template lookup.

pub trait Helpers {
    /// Reads what vanilla would place for a structure template at a position, without placing it.
    /// Returns its transformed size and blocks in placement order, including saved block entity
    /// data. The world's saved templates take precedence over data packs; missing or invalid ids
    /// return None. No processors run and entities are omitted.
    #[helper(
        side = "server",
        target = "horizon_client::game::native_structures::Game<'_>::native_structure_template"
    )]
    fn structure_template();
}
