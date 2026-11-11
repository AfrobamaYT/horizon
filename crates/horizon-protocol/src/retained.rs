//! Retained allocation charges, excluding inline packet storage and allocator metadata.
//! Vec/String/Box storage uses actual capacity/layout. Shared Arc storage is charged in
//! full per reference (safe overcount), never scanned through unrelated registries.
use std::{collections::BTreeMap, sync::Arc};
/// Required owned-allocation traversal. There is deliberately no default implementation:
/// adding a field with a new representation must supply its accounting policy.
pub trait OwnedHeap {
    fn owned_heap_bytes(&self) -> usize;
}
macro_rules! scalar { ($($t:ty),*) => { $(impl OwnedHeap for $t { fn owned_heap_bytes(&self) -> usize { 0 } })* }; }
scalar!(
    (),
    bool,
    u8,
    i8,
    u16,
    i16,
    u32,
    i32,
    u64,
    i64,
    u128,
    i128,
    usize,
    isize,
    f32,
    f64
);
impl OwnedHeap for String {
    fn owned_heap_bytes(&self) -> usize {
        self.capacity()
    }
}
impl<T: OwnedHeap> OwnedHeap for [T] {
    fn owned_heap_bytes(&self) -> usize {
        self.iter()
            .fold(0usize, |sum, v| sum.saturating_add(v.owned_heap_bytes()))
    }
}
impl<T: OwnedHeap, const N: usize> OwnedHeap for [T; N] {
    fn owned_heap_bytes(&self) -> usize {
        self.as_slice().owned_heap_bytes()
    }
}
impl<T: OwnedHeap> OwnedHeap for Vec<T> {
    fn owned_heap_bytes(&self) -> usize {
        self.capacity()
            .saturating_mul(size_of::<T>())
            .saturating_add(self.as_slice().owned_heap_bytes())
    }
}
impl<T: OwnedHeap> OwnedHeap for Option<T> {
    fn owned_heap_bytes(&self) -> usize {
        self.as_ref().map_or(0, OwnedHeap::owned_heap_bytes)
    }
}
impl<T: OwnedHeap + ?Sized> OwnedHeap for Box<T> {
    fn owned_heap_bytes(&self) -> usize {
        size_of_val(&**self).saturating_add((**self).owned_heap_bytes())
    }
}
impl<T: OwnedHeap + ?Sized> OwnedHeap for Arc<T> {
    fn owned_heap_bytes(&self) -> usize {
        size_of_val(&**self)
            .saturating_add(2 * size_of::<usize>())
            .saturating_add(align_of_val(&**self) - 1)
            .saturating_add((**self).owned_heap_bytes())
    }
}
macro_rules! tuple { ($($t:ident:$i:tt),*) => { impl<$($t:OwnedHeap),*> OwnedHeap for ($($t,)*) { fn owned_heap_bytes(&self) -> usize { 0usize $(.saturating_add(self.$i.owned_heap_bytes()))* } } }; }
tuple!(A:0,B:1);
tuple!(A:0,B:1,C:2);
tuple!(A:0,B:1,C:2,D:3);
tuple!(A:0,B:1,C:2,D:3,E:4);
impl OwnedHeap for horizon_core::nbt::JavaString {
    fn owned_heap_bytes(&self) -> usize {
        self.0.owned_heap_bytes()
    }
}
impl OwnedHeap for horizon_core::nbt::Tag {
    fn owned_heap_bytes(&self) -> usize {
        use horizon_core::nbt::Tag::*;
        match self {
            ByteArray(v) => v.owned_heap_bytes(),
            String(v) => v.owned_heap_bytes(),
            List(v) => v.owned_heap_bytes(),
            Compound(v) => v.owned_heap_bytes(),
            IntArray(v) => v.owned_heap_bytes(),
            LongArray(v) => v.owned_heap_bytes(),
            End | Byte(_) | Short(_) | Int(_) | Long(_) | Float(_) | Double(_) => 0,
        }
    }
}
impl<K: OwnedHeap, V: OwnedHeap> OwnedHeap for BTreeMap<K, V> {
    fn owned_heap_bytes(&self) -> usize {
        // Pinned Rust 1.99.1 alloc/collections/btree/node.rs: B=6, capacity=11,
        // non-root minimum=5, leaf header=(parent pointer,u16 index,u16 len),
        // internal nodes add 12 child pointers. Charge every node as internal.
        // At most 1 + len/5 nodes, including a possibly retained empty root.
        // Eight alignment gaps bound both layouts regardless of Rust field ordering.
        let align = align_of::<K>()
            .max(align_of::<V>())
            .max(align_of::<usize>());
        let node = 13usize
            .saturating_mul(size_of::<usize>())
            .saturating_add(4)
            .saturating_add(11usize.saturating_mul(size_of::<K>().saturating_add(size_of::<V>())))
            .saturating_add(8usize.saturating_mul(align - 1));
        self.iter()
            .fold((1 + self.len() / 5).saturating_mul(node), |sum, (k, v)| {
                sum.saturating_add(k.owned_heap_bytes())
                    .saturating_add(v.owned_heap_bytes())
            })
    }
}
