//! A separate test binary owns this allocator. The unique tag isolates the
//! retained values from the harness and the allocations made by census() itself.
use horizon_core::nbt::{JavaString, Tag};
use horizon_protocol::retained::OwnedHeap;
use horizon_threads::{CensusAllocator, heap_census};
use std::collections::BTreeMap;

#[global_allocator]
static ALLOCATOR: CensusAllocator = CensusAllocator;
const TAG: &str = "protocol retained map census";

fn live_bytes() -> usize {
    let bytes = heap_census::census()
        .into_iter()
        .find_map(|(name, bytes)| (name == TAG).then_some(bytes))
        .unwrap_or(0);
    usize::try_from(bytes).expect("tag's live allocation count must not be negative")
}

fn assert_covers_live_layout<T: OwnedHeap>(case: &str, build: impl FnOnce() -> T) {
    assert_eq!(live_bytes(), 0, "previous case leaked tagged storage");
    let value = {
        // Direct tag() is unconditional: the alloc-census feature is unnecessary
        // when explicitly installing CensusAllocator instead of the game's allocator.
        let _tag = heap_census::tag(TAG);
        build()
    };
    // The tag guard is gone before census() allocates its own result vector.
    // CensusAllocator records Layout::size(), excluding its own header and the
    // backing allocator's metadata/rounding, exactly the accounting boundary here.
    let observed = live_bytes();
    let charged = value.owned_heap_bytes();
    assert!(
        observed > 0,
        "{case}: census must observe real retained storage"
    );
    assert!(
        charged >= observed,
        "{case}: charge {charged} undercounts {observed} live requested bytes"
    );
    drop(value);
    assert_eq!(
        live_bytes(),
        0,
        "{case}: dropping the value must release every tagged allocation"
    );
}

#[test]
fn opaque_map_charges_cover_independently_observed_live_layouts() {
    assert_covers_live_layout("populated NBT compound", || {
        (0..4096)
            .map(|i| (JavaString::from(i.to_string().as_str()), Tag::Int(i)))
            .collect::<BTreeMap<_, _>>()
    });
    assert_covers_live_layout("nested maps and spare child capacities", || {
        let mut outer = BTreeMap::new();
        for i in 0..64 {
            let mut inner = BTreeMap::new();
            for j in 0..31 {
                let mut units = Vec::with_capacity(100);
                units.push(j);
                let mut bytes = Vec::with_capacity(1000);
                bytes.push(42);
                let mut children = Vec::with_capacity(20);
                children.push(Tag::String(JavaString(units)));
                children.push(Tag::ByteArray(bytes));
                inner.insert(
                    JavaString::from(j.to_string().as_str()),
                    Tag::List(children),
                );
            }
            outer.insert(
                JavaString::from(i.to_string().as_str()),
                Tag::Compound(inner),
            );
        }
        Tag::Compound(outer)
    });
    assert_covers_live_layout("insert/remove churn", || {
        let mut map = BTreeMap::new();
        for i in (0..4096).rev() {
            map.insert(JavaString::from(i.to_string().as_str()), Tag::Int(i));
        }
        for i in 0..4096 {
            if i % 11 != 0 {
                map.remove(&JavaString::from(i.to_string().as_str()));
            }
        }
        map
    });
    assert_covers_live_layout("retained empty root", || {
        let mut map = BTreeMap::<u32, [u64; 32]>::new();
        for i in 0..1000 {
            map.insert(i, [0; 32]);
        }
        for i in 0..1000 {
            map.remove(&i);
        }
        map
    });
}
