//! Dense attachment membership with immutable, shared-entry snapshots.
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::Arc,
};

use super::Entry;
use crate::nbt::JavaString;

/// An already-resolved attachment namespace and name.
pub type AttachmentKey = (JavaString, JavaString);

/// A loaded holder and the immutable value it had when a snapshot was acquired.
#[derive(Clone)]
#[non_exhaustive]
pub struct AttachmentRow<H> {
    /// The ready-to-use holder identity; iteration needs no lookup or conversion.
    pub holder: H,
    /// Shared with the holder's attachment map, never copied by snapshot acquisition.
    pub entry: Arc<Entry>,
}

/// Authority to publish one registration's changes. Cloning or dropping this token has
/// no membership effect. Re-registering the holder invalidates all earlier tokens.
#[derive(Clone)]
#[non_exhaustive]
pub struct AttachmentOwner<H> {
    holder: H,
    generation: Arc<()>,
}

struct Owner {
    generation: Arc<()>,
    keys: HashSet<AttachmentKey>,
}

struct Bucket<H> {
    rows: Arc<Vec<AttachmentRow<H>>>,
    slots: HashMap<H, usize>,
}

impl<H: Copy + Eq + Hash> Bucket<H> {
    fn set(&mut self, holder: H, entry: Arc<Entry>) {
        let rows = Arc::make_mut(&mut self.rows);
        let row = AttachmentRow { holder, entry };
        if let Some(&slot) = self.slots.get(&holder) {
            rows[slot] = row;
        } else {
            self.slots.insert(holder, rows.len());
            rows.push(row);
        }
    }

    fn remove(&mut self, holder: H) {
        let Some(slot) = self.slots.remove(&holder) else {
            return;
        };
        let rows = Arc::make_mut(&mut self.rows);
        rows.swap_remove(slot);
        if let Some(moved) = rows.get(slot) {
            self.slots.insert(moved.holder, slot);
        }
    }
}

/// A dimension's loaded attachment memberships, independent of holder storage.
///
/// Snapshots clone one Arc, and borrowed row iteration needs no locks or allocations.
/// Mutation copies a bucket's row vector only when a snapshot retains that version;
/// entries themselves remain shared. With no retained version, updates are in place.
/// All key/holder lookups are expected O(1). Unregistration visits only that holder's
/// keys, not all types. The owner registration is an explicit lifecycle operation,
/// never an effect of dropping a map or a token.
///
/// Rows append on first insertion. Replacement preserves the slot; removal fills it
/// from the final row. Order is deterministic given the ordered registration/mutation
/// history, but is neither sorted nor stable across removals. Callers restoring many
/// holders must supply a defined registration order rather than HashMap iteration.
#[non_exhaustive]
pub struct AttachmentIndex<H> {
    buckets: HashMap<AttachmentKey, Bucket<H>>,
    owners: HashMap<H, Owner>,
    empty: Arc<Vec<AttachmentRow<H>>>,
}

impl<H> Default for AttachmentIndex<H> {
    fn default() -> Self {
        Self {
            buckets: HashMap::new(),
            owners: HashMap::new(),
            empty: Arc::new(Vec::new()),
        }
    }
}

impl<H: Copy + Eq + Hash> AttachmentIndex<H> {
    /// Install the authoritative map, replacing the holder's previous membership.
    /// Only lifecycle code may register; a bound map publishes through its token.
    /// Rebinding invalidates stale save/COW bindings, even when their keys match.
    pub fn register(
        &mut self,
        holder: H,
        entries: impl IntoIterator<Item = (AttachmentKey, Arc<Entry>)>,
    ) -> AttachmentOwner<H> {
        if let Some(previous) = self.owners.remove(&holder) {
            self.remove_memberships(holder, previous.keys);
        }
        // A live stale token retains its identity, so generations cannot wrap or be
        // reused, including across distinct indexes.
        let generation = Arc::new(());
        let token = AttachmentOwner { holder, generation };
        self.owners.insert(
            holder,
            Owner {
                generation: token.generation.clone(),
                keys: HashSet::new(),
            },
        );
        for (key, entry) in entries {
            self.set(&token, &key, entry);
        }
        token
    }

    /// Capture current membership and values. Missing keys share the same empty Arc;
    /// acquiring an empty snapshot neither allocates nor registers the key.
    pub fn snapshot(&self, key: &AttachmentKey) -> Arc<Vec<AttachmentRow<H>>> {
        self.buckets
            .get(key)
            .map_or(&self.empty, |bucket| &bucket.rows)
            .clone()
    }

    /// Publish insertion or replacement. Return false for a stale or foreign token,
    /// leaving the index unchanged. Replacement does not reorder the holder's row.
    pub fn set(
        &mut self,
        token: &AttachmentOwner<H>,
        key: &AttachmentKey,
        entry: Arc<Entry>,
    ) -> bool {
        let Some(owner) = self.owners.get_mut(&token.holder) else {
            return false;
        };
        if !Arc::ptr_eq(&owner.generation, &token.generation) {
            return false;
        }
        if !owner.keys.contains(key) {
            owner.keys.insert(key.clone());
        }
        match self.buckets.get_mut(key) {
            Some(bucket) => bucket.set(token.holder, entry),
            None => {
                let mut bucket = Bucket {
                    rows: Arc::new(Vec::new()),
                    slots: HashMap::new(),
                };
                bucket.set(token.holder, entry);
                self.buckets.insert(key.clone(), bucket);
            }
        }
        true
    }

    /// Remove one membership. Return false only for a stale or foreign token.
    pub fn remove(&mut self, token: &AttachmentOwner<H>, key: &AttachmentKey) -> bool {
        let Some(owner) = self.owners.get_mut(&token.holder) else {
            return false;
        };
        if !Arc::ptr_eq(&owner.generation, &token.generation) {
            return false;
        }
        if owner.keys.remove(key) {
            self.remove_membership(token.holder, key);
        }
        true
    }

    /// Explicitly unload this registration. Stale or foreign tokens cannot unload
    /// a replacement owner. Retained snapshots keep their original rows and values.
    pub fn unregister(&mut self, token: &AttachmentOwner<H>) -> bool {
        let Some(owner) = self.owners.get(&token.holder) else {
            return false;
        };
        if !Arc::ptr_eq(&owner.generation, &token.generation) {
            return false;
        }
        let owner = self.owners.remove(&token.holder).unwrap();
        self.remove_memberships(token.holder, owner.keys);
        true
    }

    fn remove_memberships(&mut self, holder: H, keys: HashSet<AttachmentKey>) {
        for key in keys {
            self.remove_membership(holder, &key);
        }
    }

    fn remove_membership(&mut self, holder: H, key: &AttachmentKey) {
        let bucket = self.buckets.get_mut(key).unwrap();
        bucket.remove(holder);
        if bucket.slots.is_empty() {
            self.buckets.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachments::{AttachmentCodec, AttachmentType, Attachments, Holder};
    use crate::nbt::{Compound, Tag};

    static VALUE: AttachmentType<i32> = AttachmentType {
        namespace: "test",
        name: "value",
        default: || 0,
        codec: Some(AttachmentCodec {
            encode: |n| Ok([("value".into(), Tag::Int(*n))].into()),
            decode: |_| unreachable!(),
        }),
        copy_on_death: false,
        sync: false,
    };

    fn key(name: &str) -> AttachmentKey {
        ("test".into(), name.into())
    }

    fn entry(n: i32) -> Arc<Entry> {
        let mut map = Attachments::default();
        VALUE.set(&mut map, Holder::Level, n).unwrap();
        map.entries.values().next().unwrap().clone()
    }

    fn values(rows: &[AttachmentRow<u32>]) -> Vec<(u32, i32)> {
        rows.iter()
            .map(|row| {
                let form: &Compound = row.entry.synced_value().unwrap();
                let Tag::Int(n) = form[&"value".into()] else {
                    panic!("not an integer")
                };
                (row.holder, n)
            })
            .collect()
    }

    #[test]
    fn snapshots_keep_membership_and_values_while_current_rows_change() {
        let mut index = AttachmentIndex::default();
        let original = entry(10);
        let a = index.register(1, [(key("value"), original.clone())]);
        let b = index.register(2, [(key("value"), entry(20))]);
        let before = index.snapshot(&key("value"));
        assert!(Arc::ptr_eq(&before[0].entry, &original));
        assert!(index.set(&a, &key("value"), entry(11)));
        assert!(index.remove(&b, &key("value")));
        index.register(3, [(key("value"), entry(30))]);
        assert_eq!(values(&before), [(1, 10), (2, 20)]);
        assert_eq!(values(&index.snapshot(&key("value"))), [(1, 11), (3, 30)]);
    }

    #[test]
    fn writes_copy_rows_once_per_retained_version_and_keep_other_entries_shared() {
        let mut index = AttachmentIndex::default();
        let a = index.register(1, [(key("value"), entry(1))]);
        let untouched = entry(2);
        index.register(2, [(key("value"), untouched.clone())]);
        let before = index.snapshot(&key("value"));
        index.set(&a, &key("value"), entry(10));
        let current = index.snapshot(&key("value"));
        assert!(!Arc::ptr_eq(&before, &current));
        assert!(Arc::ptr_eq(&untouched, &current[1].entry));
        let current_version = Arc::as_ptr(&current);
        drop(current);
        index.set(&a, &key("value"), entry(11));
        let after = index.snapshot(&key("value"));
        assert_eq!(Arc::as_ptr(&after), current_version);
        assert_eq!(values(&before), [(1, 1), (2, 2)]);
        assert_eq!(values(&after), [(1, 11), (2, 2)]);
    }

    #[test]
    fn swap_removal_repairs_slots_and_replacement_preserves_order() {
        let mut index = AttachmentIndex::default();
        let a = index.register(1, [(key("value"), entry(1))]);
        let b = index.register(2, [(key("value"), entry(2))]);
        let c = index.register(3, [(key("value"), entry(3))]);
        let d = index.register(4, [(key("value"), entry(4))]);
        assert!(index.remove(&b, &key("value")));
        assert!(index.set(&d, &key("value"), entry(40)));
        assert_eq!(
            values(&index.snapshot(&key("value"))),
            [(1, 1), (4, 40), (3, 3)]
        );
        assert!(index.unregister(&a));
        assert!(index.set(&c, &key("value"), entry(30)));
        assert_eq!(values(&index.snapshot(&key("value"))), [(3, 30), (4, 40)]);
        assert!(index.remove(&d, &key("value")));
        assert_eq!(values(&index.snapshot(&key("value"))), [(3, 30)]);
    }

    #[test]
    fn rebind_rejects_stale_tokens_and_unloads_only_the_owners_keys() {
        let mut index = AttachmentIndex::default();
        let old = index.register(1, [(key("a"), entry(1)), (key("b"), entry(2))]);
        index.register(2, [(key("b"), entry(20))]);
        let retained = index.snapshot(&key("a"));
        let current = index.register(1, [(key("c"), entry(3))]);
        assert!(!index.set(&old, &key("c"), entry(99)));
        assert!(!index.remove(&old, &key("c")));
        assert!(!index.unregister(&old));
        assert_eq!(values(&retained), [(1, 1)]);
        assert!(index.snapshot(&key("a")).is_empty());
        assert_eq!(values(&index.snapshot(&key("b"))), [(2, 20)]);
        assert_eq!(values(&index.snapshot(&key("c"))), [(1, 3)]);
        let clone = current.clone();
        drop(current);
        assert_eq!(values(&index.snapshot(&key("c"))), [(1, 3)]);
        assert!(index.unregister(&clone));
        assert!(!index.set(&clone, &key("a"), entry(9)));
        assert_eq!(values(&index.snapshot(&key("b"))), [(2, 20)]);
    }

    #[test]
    fn tokens_from_another_index_cannot_publish_or_unload() {
        let mut first = AttachmentIndex::default();
        let foreign = first.register(1, [(key("value"), entry(1))]);
        let mut second = AttachmentIndex::default();
        second.register(1, [(key("value"), entry(2))]);
        assert!(!second.set(&foreign, &key("value"), entry(99)));
        assert!(!second.remove(&foreign, &key("value")));
        assert!(!second.unregister(&foreign));
        assert_eq!(values(&second.snapshot(&key("value"))), [(1, 2)]);
    }

    #[test]
    fn missing_keys_reuse_empty_snapshot_and_do_not_become_memberships() {
        let mut index = AttachmentIndex::<u32>::default();
        let empty = index.snapshot(&key("missing"));
        assert!(empty.is_empty());
        assert!(Arc::ptr_eq(&empty, &index.snapshot(&key("other"))));
        let owner = index.register(1, []);
        assert!(index.remove(&owner, &key("missing")));
        assert!(index.unregister(&owner));
        assert!(Arc::ptr_eq(&empty, &index.snapshot(&key("missing"))));
        index.register(1, [(key("missing"), entry(7)), (key("missing"), entry(8))]);
        assert_eq!(values(&index.snapshot(&key("missing"))), [(1, 8)]);
        assert!(empty.is_empty());
    }
}
