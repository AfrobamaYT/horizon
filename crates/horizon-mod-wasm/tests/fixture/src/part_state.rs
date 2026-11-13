//! Block entity Part state (#781), shared by the native and the compiled wasm run of the game
//! test: each tick counts in the state, after an aborted edit and a refused recursive one.
use mod_api::*;

pub static COUNT: AttachmentType<i32> = AttachmentType {
    namespace: "wasm_fixture",
    name: "part_state_count",
    default: || 0,
    codec: Some(AttachmentCodec {
        encode: |n| Ok([("n".into(), Tag::Int(*n))].into()),
        decode: |saved| match saved.get(&"n".into()) {
            Some(Tag::Int(n)) => Ok(*n),
            _ => Err(AttachmentError::Decode),
        },
    }),
    copy_on_death: false,
    sync: false,
};

pub struct Counter;
#[mod_api::horizon_part]
impl BlockPart for Counter {}
#[mod_api::horizon_part]
impl BlockEntityPart for Counter {
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let level = host.level(host.dimension()).unwrap();
        let entity = host.block_entity_identity(level, event.pos).unwrap();
        let aborted: Result<(), ()> = host.transaction(|host, _| {
            assert_eq!(Counter::edit_state(host, entity, |_, n| *n = 100), Ok(()));
            Err(())
        });
        assert_eq!(aborted, Err(()));
        let recursive = Counter::edit_state(host, entity, |host, n| {
            *n += 1;
            Counter::edit_state(host, entity, |_, n| *n = 50)
        });
        assert_eq!(recursive, Ok(Err(PartStateError::Reentrant)));
    }
}
impl BlockEntityPartState for Counter {
    type Data = i32;
    const DATA: &'static AttachmentType<i32> = &COUNT;
}

pub fn register(registrar: &mut dyn Registrar) {
    let kind = registrar
        .block_entity_type("part_state")
        .blocks(&[])
        .build();
    registrar
        .block_entity_part_with_state(&kind, Counter)
        .unwrap();
}
