use super::*;
fn holder() -> BlockEntity {
    BlockEntity::Mod(Box::new(crate::containers::ModBlockEntity::new(
        "test:holder".into(),
        Default::default(),
    )))
}
#[test]
fn on_load_tracks_identity_and_skips_vanilla_and_retired_instances() {
    let mut entities = BlockEntities::default();
    let pos = (1, 64, 1);
    entities.insert(pos, holder());
    let generation = entities.generation(&pos).unwrap();
    assert_eq!(entities.next_on_load(), Some((pos, generation)));
    entities.insert(pos, holder());
    assert_eq!(entities.next_on_load(), None);
    let (lease, entity) = entities.take(&pos).unwrap();
    assert!(entities.put(lease, entity));
    assert_eq!(entities.next_on_load(), None);
    entities.insert((2, 64, 1), holder());
    entities.remove(&(2, 64, 1));
    entities.insert((3, 64, 1), BlockEntity::Chest(vec![]));
    assert_eq!(entities.next_on_load(), None);
    entities.load_chunk(vec![(pos, holder())]);
    let replacement = entities.generation(&pos).unwrap();
    assert_ne!(generation, replacement);
    assert_eq!(entities.next_on_load(), Some((pos, replacement)));
    assert_eq!(entities.next_on_load(), None);
}
