//! Checked wire decoding for opaque API values.
use crate::wire::{Reader, Wire, WireError, Writer};
use crate::{
    BlockEntityTypeKey, BlockKey, BoolGameRuleKey, DimensionKey, EffectKey, FluidKey, ItemKey,
    TagKey, UnresolvedFluid,
};

macro_rules! slot_keys {
    ($width:ty; $($key:ty),+ $(,)?) => {$ (
        impl Wire for $key {
            const MIN: usize = <$width as Wire>::MIN;
            fn put(&self, out: &mut Writer) { out.put(&self.0); }
            fn take(input: &mut Reader<'_>) -> Result<Self, WireError> {
                Self::from_slot(input.take::<$width>()?).map_err(WireError::Invalid)
            }
        }
    )+};
}
slot_keys!(u32; BlockEntityTypeKey, BlockKey, BoolGameRuleKey, DimensionKey, EffectKey, FluidKey, ItemKey, TagKey);
slot_keys!(u64; UnresolvedFluid);

impl Wire for crate::NativeWorldEpoch {
    const MIN: usize = 8;
    fn put(&self, out: &mut Writer) {
        out.put(&self.get());
    }
    fn take(input: &mut Reader<'_>) -> Result<Self, WireError> {
        Ok(Self::new(input.take()?))
    }
}
impl Wire for crate::NativeTickId {
    const MIN: usize = 8;
    fn put(&self, out: &mut Writer) {
        out.put(&self.get());
    }
    fn take(input: &mut Reader<'_>) -> Result<Self, WireError> {
        Ok(Self::new(input.take()?))
    }
}
impl Wire for crate::EntityRef {
    const MIN: usize = 12;
    fn put(&self, out: &mut Writer) {
        out.put(&self.epoch());
        out.put(&self.id());
    }
    fn take(input: &mut Reader<'_>) -> Result<Self, WireError> {
        Ok(Self::new(input.take()?, input.take()?))
    }
}
macro_rules! entity_handles {
    ($($ty:ty),+ $(,)?) => { $(
        impl Wire for $ty {
            const MIN: usize = crate::EntityRef::MIN;
            fn put(&self, out: &mut Writer) { out.put(&self.entity()); }
            fn take(input: &mut Reader<'_>) -> Result<Self, WireError> { Ok(Self::new(input.take()?)) }
        }
    )+ };
}
entity_handles!(
    crate::LivingRef,
    crate::MobRef,
    crate::DragonRef,
    crate::ItemEntityRef,
    crate::DisplayRef,
    crate::ProjectileRef,
    crate::MerchantRef,
    crate::MinecartRef
);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_dimension_key_checked() {
        let mut out = Writer::new();
        out.put(&DimensionKey::OVERWORLD);
        assert_eq!(
            Reader::decode::<DimensionKey>(out.bytes()),
            Ok(DimensionKey::OVERWORLD)
        );
        assert!(Reader::decode::<DimensionKey>(&u32::MAX.to_le_bytes()).is_err());
        assert!(Reader::decode::<DimensionKey>(&[]).is_err());
    }
}
