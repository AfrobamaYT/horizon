use mod_api::Hooks;

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, registrar: &mut dyn mod_api::Registrar, _: &mod_api::RegistrationContext) {
        let kind = registrar.block_entity_type("counter").blocks(&[]).build();
        registrar.block_entity_part(&kind, Counter).unwrap();
    }
}

pub struct Counter;

#[mod_api::horizon_part]
impl mod_api::BlockPart for Counter {}

#[mod_api::horizon_part]
impl mod_api::BlockEntityPart for Counter {
    fn block_entity_tick(
        &self,
        _: &mut dyn mod_api::ServerHost,
        event: &mut mod_api::BlockEntityTickEvent<'_>,
    ) {
        event
            .entity
            .nbt
            .insert("counter".into(), mod_api::Tag::Int(17));
        event.entity.data = vec![23];
        event.entity.items.clear();
    }
}
