//! The hand-written wire forms of the types that cross whole: handles as their records,
//! NBT and stacks as network NBT bytes, text as its JSON component. One macro, expanded on
//! each side against that side's bindings (`$w`, the module of the `types` interface).

macro_rules! codecs {
    ($w:path) => {
        use $w as w;
        use crate::{
            BlockEntityRef, BossBarRef, ChunkRef, DisplayRef, DragonRef, EntityRef, ItemEntityRef,
            LevelRef, LivingRef, MerchantRef, MinecartRef, MobRef, NativeBlockUpdateFlags,
            NativeTickId, NativeWorldEpoch, PlayerRef, ProjectileRef, RaidRef,
        };
        use horizon_client_core::text::Text;
        use horizon_items::items::ItemStack;
        use horizon_world::identifier::Identifier;
        use horizon_world::nbt::{Compound, Limits, Tag};

        fn u128_of((high, low): (u64, u64)) -> u128 {
            (high as u128) << 64 | low as u128
        }
        fn split(v: u128) -> (u64, u64) {
            ((v >> 64) as u64, v as u64)
        }

        pub(super) fn lower_native_world_epoch(v: &NativeWorldEpoch) -> u64 {
            v.0
        }
        pub(super) fn lift_native_world_epoch(v: u64) -> Result<NativeWorldEpoch, Refused> {
            Ok(NativeWorldEpoch(v))
        }
        pub(super) fn lower_native_tick_id(v: &NativeTickId) -> u64 {
            v.0
        }
        pub(super) fn lift_native_tick_id(v: u64) -> Result<NativeTickId, Refused> {
            Ok(NativeTickId(v))
        }
        pub(super) fn lower_native_block_update_flags(v: &NativeBlockUpdateFlags) -> u32 {
            v.0 as u32
        }
        pub(super) fn lift_native_block_update_flags(v: u32) -> Result<NativeBlockUpdateFlags, Refused> {
            Ok(NativeBlockUpdateFlags(v as i32))
        }
        pub(super) fn lower_duration(v: &std::time::Duration) -> u64 {
            u64::try_from(v.as_nanos()).unwrap_or(u64::MAX)
        }
        pub(super) fn lift_duration(v: u64) -> Result<std::time::Duration, Refused> {
            Ok(std::time::Duration::from_nanos(v))
        }
        pub(super) fn lower_entity_ref(v: &EntityRef) -> w::EntityRef {
            w::EntityRef { epoch: v.epoch.0, id: v.id }
        }
        pub(super) fn lift_entity_ref(v: w::EntityRef) -> Result<EntityRef, Refused> {
            Ok(EntityRef::new(NativeWorldEpoch(v.epoch), v.id))
        }
        pub(super) fn lower_living_ref(v: &LivingRef) -> w::EntityRef {
            lower_entity_ref(&v.0)
        }
        pub(super) fn lift_living_ref(v: w::EntityRef) -> Result<LivingRef, Refused> {
            Ok(LivingRef(lift_entity_ref(v)?))
        }
        pub(super) fn lower_mob_ref(v: &MobRef) -> w::EntityRef {
            lower_entity_ref(&v.0)
        }
        pub(super) fn lift_mob_ref(v: w::EntityRef) -> Result<MobRef, Refused> {
            Ok(MobRef(lift_entity_ref(v)?))
        }
        pub(super) fn lower_display_ref(v: &DisplayRef) -> w::EntityRef {
            lower_entity_ref(&v.0)
        }
        pub(super) fn lift_display_ref(v: w::EntityRef) -> Result<DisplayRef, Refused> {
            Ok(DisplayRef(lift_entity_ref(v)?))
        }
        pub(super) fn lower_item_entity_ref(v: &ItemEntityRef) -> w::EntityRef {
            lower_entity_ref(&v.0)
        }
        pub(super) fn lift_item_entity_ref(v: w::EntityRef) -> Result<ItemEntityRef, Refused> {
            Ok(ItemEntityRef(lift_entity_ref(v)?))
        }
        pub(super) fn lower_projectile_ref(v: &ProjectileRef) -> w::EntityRef {
            lower_entity_ref(&v.0)
        }
        pub(super) fn lift_projectile_ref(v: w::EntityRef) -> Result<ProjectileRef, Refused> {
            Ok(ProjectileRef(lift_entity_ref(v)?))
        }
        pub(super) fn lower_minecart_ref(v: &MinecartRef) -> w::EntityRef {
            lower_entity_ref(&v.0)
        }
        pub(super) fn lift_minecart_ref(v: w::EntityRef) -> Result<MinecartRef, Refused> {
            Ok(MinecartRef(lift_entity_ref(v)?))
        }
        pub(super) fn lower_dragon_ref(v: &DragonRef) -> w::EntityRef {
            lower_entity_ref(&v.0)
        }
        pub(super) fn lift_dragon_ref(v: w::EntityRef) -> Result<DragonRef, Refused> {
            Ok(DragonRef(lift_entity_ref(v)?))
        }
        pub(super) fn lower_merchant_ref(v: &MerchantRef) -> w::EntityRef {
            lower_entity_ref(&v.0)
        }
        pub(super) fn lift_merchant_ref(v: w::EntityRef) -> Result<MerchantRef, Refused> {
            Ok(MerchantRef(lift_entity_ref(v)?))
        }
        pub(super) fn lower_player_ref(v: &PlayerRef) -> w::PlayerRef {
            w::PlayerRef { entity: lower_entity_ref(&v.entity), uuid: split(v.uuid) }
        }
        pub(super) fn lift_player_ref(v: w::PlayerRef) -> Result<PlayerRef, Refused> {
            Ok(PlayerRef { entity: lift_entity_ref(v.entity)?, uuid: u128_of(v.uuid) })
        }
        pub(super) fn lower_level_ref(v: &LevelRef) -> w::LevelRef {
            w::LevelRef { epoch: v.epoch.0, dimension: v.dimension.to_string() }
        }
        pub(super) fn lift_level_ref(v: w::LevelRef) -> Result<LevelRef, Refused> {
            Ok(LevelRef::new(NativeWorldEpoch(v.epoch), super::intern(v.dimension)))
        }
        pub(super) fn lower_chunk_ref(v: &ChunkRef) -> w::ChunkRef {
            w::ChunkRef { level: lower_level_ref(&v.level), pos: (v.pos[0], v.pos[1]) }
        }
        pub(super) fn lift_chunk_ref(v: w::ChunkRef) -> Result<ChunkRef, Refused> {
            Ok(ChunkRef { level: lift_level_ref(v.level)?, pos: [v.pos.0, v.pos.1] })
        }
        pub(super) fn lower_block_entity_ref(v: &BlockEntityRef) -> w::BlockEntityRef {
            let [x, y, z] = v.position;
            w::BlockEntityRef { level: lower_level_ref(&v.level), pos: (x, y, z) }
        }
        pub(super) fn lift_block_entity_ref(v: w::BlockEntityRef) -> Result<BlockEntityRef, Refused> {
            let (x, y, z) = v.pos;
            Ok(BlockEntityRef { level: lift_level_ref(v.level)?, position: [x, y, z] })
        }
        pub(super) fn lower_boss_bar_ref(v: &BossBarRef) -> w::BossBarRef {
            w::BossBarRef { epoch: v.epoch.0, id: split(v.uuid) }
        }
        pub(super) fn lift_boss_bar_ref(v: w::BossBarRef) -> Result<BossBarRef, Refused> {
            Ok(BossBarRef { epoch: NativeWorldEpoch(v.epoch), uuid: u128_of(v.id) })
        }
        pub(super) fn lower_raid_ref(v: &RaidRef) -> w::RaidRef {
            w::RaidRef { level: lower_level_ref(&v.level), id: v.id }
        }
        pub(super) fn lift_raid_ref(v: w::RaidRef) -> Result<RaidRef, Refused> {
            Ok(RaidRef { level: lift_level_ref(v.level)?, id: v.id })
        }
        pub(super) fn lower_tag(v: &Tag) -> Vec<u8> {
            horizon_world::nbt::write_network(v, Limits::default()).unwrap_or_default()
        }
        pub(super) fn lift_tag(v: Vec<u8>) -> Result<Tag, Refused> {
            horizon_world::nbt::read_network(&v, Limits::default())
                .map(|(tag, _)| tag)
                .map_err(|_| Refused)
        }
        pub(super) fn lower_compound(v: &Compound) -> Vec<u8> {
            lower_tag(&Tag::Compound(v.clone()))
        }
        pub(super) fn lift_compound(v: Vec<u8>) -> Result<Compound, Refused> {
            match lift_tag(v)? {
                Tag::Compound(compound) => Ok(compound),
                _ => Err(Refused),
            }
        }
        pub(super) fn lower_item_stack(v: &ItemStack) -> Vec<u8> {
            lower_tag(&horizon_items::item_nbt::item_to_nbt(v))
        }
        pub(super) fn lift_item_stack(v: Vec<u8>) -> Result<ItemStack, Refused> {
            horizon_items::item_nbt::item_from_nbt(&lift_tag(v)?).ok_or(Refused)
        }
        pub(super) fn lower_text(v: &Text) -> String {
            v.component().to_string()
        }
        pub(super) fn lift_text(v: String) -> Result<Text, Refused> {
            let value: serde_json::Value = serde_json::from_str(&v).map_err(|_| Refused)?;
            Ok(horizon_client_core::component::to_text(&value, &mut |s| Text::literal(s))
                .with_component(value))
        }
        pub(super) fn lower_identifier(v: &Identifier) -> String {
            v.to_string()
        }
        pub(super) fn lift_identifier(v: String) -> Result<Identifier, Refused> {
            Identifier::parse(&v).map_err(|_| Refused)
        }
    };
}
