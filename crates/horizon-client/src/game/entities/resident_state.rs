//! Game-owned residency state; no second membership directory.
use super::*;
pub(super) struct ResidentEntities {
    pub table: Box<RefCell<horizon_client::join_seq::residency::table::EntityTable>>,
    pub local_player: horizon_client::join_seq::residency::member::Member,
    pub knots: horizon_client::join_seq::JoinList<leads::Knot>,
}

impl Default for ResidentEntities {
    fn default() -> Self {
        Self {
            table: Box::new(RefCell::new(
                horizon_client::join_seq::residency::table::EntityTable::new(Default::default()),
            )),
            local_player: Default::default(),
            knots: Default::default(),
        }
    }
}
