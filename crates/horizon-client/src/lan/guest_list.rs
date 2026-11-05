//! Ordered guests with indexes for connection and exact player identity.
use super::guest::Guest;
use crate::fast_hash::FxHashMap;
use std::ops::{Deref, DerefMut};

/// Guest identities and kind stay fixed while listed; replace an entire guest with `replace`.
/// State mutation does not rebuild indexes. Membership changes preserve iteration order.
#[derive(Default)]
pub struct GuestList {
    rows: Vec<Guest>,
    simulation: Vec<usize>,
    connections: FxHashMap<u32, usize>,
    identities: FxHashMap<(u128, u32), usize>,
}
impl GuestList {
    pub fn connection_index(&self, conn: u32) -> Option<usize> {
        self.connections.get(&conn).copied()
    }
    pub fn identity_index(&self, uuid: u128, entity: u32) -> Option<usize> {
        self.identities.get(&(uuid, entity)).copied()
    }
    /// Material player bodies in join order, without visiting connectionless actors.
    pub fn simulation_iter(&self) -> impl Iterator<Item = &Guest> {
        self.simulation.iter().map(|&index| &self.rows[index])
    }
    pub fn as_slice(&self) -> &[Guest] {
        &self.rows
    }
    pub fn push(&mut self, guest: Guest) {
        let index = self.rows.len();
        self.connections.entry(guest.conn).or_insert(index);
        self.identities
            .entry((guest.uuid, guest.entity_id as u32))
            .or_insert(index);
        if guest.participates_in_player_simulation() {
            self.simulation.push(index);
        }
        self.rows.push(guest);
    }
    pub fn extend(&mut self, guests: impl IntoIterator<Item = Guest>) {
        for guest in guests {
            self.push(guest);
        }
    }
    pub fn remove(&mut self, index: usize) -> Guest {
        let guest = self.rows.remove(index);
        self.reindex();
        guest
    }
    pub fn retain(&mut self, keep: impl FnMut(&Guest) -> bool) {
        self.rows.retain(keep);
        self.reindex();
    }
    pub fn insert(&mut self, index: usize, guest: Guest) {
        self.rows.insert(index, guest);
        self.reindex();
    }
    pub fn replace(&mut self, index: usize, guest: Guest) -> Guest {
        let old = std::mem::replace(&mut self.rows[index], guest);
        self.reindex();
        old
    }
    pub fn clear(&mut self) {
        self.rows.clear();
        self.simulation.clear();
        self.connections.clear();
        self.identities.clear();
    }
    pub fn reverse(&mut self) {
        self.rows.reverse();
        self.reindex();
    }
    fn reindex(&mut self) {
        self.simulation.clear();
        self.connections.clear();
        self.identities.clear();
        for (index, guest) in self.rows.iter().enumerate() {
            if guest.participates_in_player_simulation() {
                self.simulation.push(index);
            }
            self.connections.entry(guest.conn).or_insert(index);
            self.identities
                .entry((guest.uuid, guest.entity_id as u32))
                .or_insert(index);
        }
    }
}
impl Deref for GuestList {
    type Target = [Guest];
    fn deref(&self) -> &Self::Target {
        &self.rows
    }
}
impl DerefMut for GuestList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rows
    }
}
impl<'a> IntoIterator for &'a GuestList {
    type Item = &'a Guest;
    type IntoIter = std::slice::Iter<'a, Guest>;
    fn into_iter(self) -> Self::IntoIter {
        self.rows.iter()
    }
}
impl<'a> IntoIterator for &'a mut GuestList {
    type Item = &'a mut Guest;
    type IntoIter = std::slice::IterMut<'a, Guest>;
    fn into_iter(self) -> Self::IntoIter {
        self.rows.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn guest(conn: u32) -> Guest {
        let (tx, _) = std::sync::mpsc::channel();
        Guest::new(
            super::super::Joined {
                conn,
                uuid: u128::from(conn) + 100,
                name: format!("guest{conn}"),
                properties: vec![],
                info: horizon_protocol::packets::configuration::ClientInformation::new(8),
                brand: "horizon".into(),
                sender: super::super::PlayerSender::new(tx, Default::default()),
                latency: Default::default(),
                address: "127.0.0.1:25565".parse().unwrap(),
                expected: false,
            },
            conn as i32 + 10,
            [0.0, 64.0, 0.0],
            8,
        )
    }
    fn check(list: &GuestList) {
        for (index, guest) in list.iter().enumerate() {
            assert_eq!(list.connection_index(guest.conn), Some(index));
            assert_eq!(
                list.identity_index(guest.uuid, guest.entity_id as u32),
                Some(index)
            );
        }
    }
    #[test]
    fn connectionless_actors_do_not_expand_the_material_body_index() {
        let mut list = GuestList::default();
        list.push(guest(1));
        for conn in 2..=257 {
            let mut fake = guest(conn);
            fake.kind = crate::lan::guest::GuestKind::Fake;
            list.push(fake);
        }
        assert_eq!(
            list.simulation_iter().map(|g| g.conn).collect::<Vec<_>>(),
            [1]
        );
        list.reverse();
        assert_eq!(
            list.simulation_iter().map(|g| g.conn).collect::<Vec<_>>(),
            [1]
        );
        list.remove(256);
        assert_eq!(list.simulation_iter().count(), 0);
        check(&list);
    }

    #[test]
    fn membership_changes_keep_both_indexes_and_iteration_order() {
        let mut list = GuestList::default();
        list.extend([guest(1), guest(2), guest(3)]);
        check(&list);
        let removed = list.remove(1);
        check(&list);
        assert_eq!(list.connection_index(2), None);
        assert_eq!(list.identity_index(102, 12), None);
        assert_eq!(list.iter().map(|g| g.conn).collect::<Vec<_>>(), [1, 3]);
        list.insert(1, removed);
        check(&list);
        list.replace(0, guest(4));
        check(&list);
        assert_eq!(list.connection_index(1), None);
        list.reverse();
        check(&list);
        list[0].placed = true;
        list[0].position[0] = 42.0;
        check(&list);
        list.retain(|g| g.conn != 2);
        check(&list);
        assert_eq!(list.connection_index(2), None);
        assert_eq!(list.identity_index(102, 12), None);
        list.clear();
        assert!(list.is_empty());
        assert_eq!(list.connection_index(3), None);
        assert_eq!(list.identity_index(103, 13), None);
    }
}
