//! Input strings retain a canonical cursor until the caller has metered the copy.
use super::*;
use wasmtime::component::__internal::wasmtime_environ::component::StringEncoding;
use wasmtime::component::__internal::{CanonicalAbiInfo, InstanceType, InterfaceType, LiftContext};
use wasmtime::component::{ComponentType, Lift, WasmStr};

pub(super) struct StringInput {
    cursor: WasmStr,
    bytes: usize,
}

pub(super) struct Input<'a, 'b> {
    pub(super) store: &'a mut wasmtime::StoreContextMut<'b, State>,
    bytes: usize,
}

impl<'a, 'b> Input<'a, 'b> {
    pub(super) fn new(store: &'a mut wasmtime::StoreContextMut<'b, State>) -> Self {
        Self { store, bytes: 0 }
    }
}

pub(super) trait Materialize: Sized {
    type Owned;
    fn materialize(self, input: &mut Input<'_, '_>) -> wasmtime::Result<Option<Self::Owned>>;
}

impl Materialize for StringInput {
    type Owned = String;
    fn materialize(self, input: &mut Input<'_, '_>) -> wasmtime::Result<Option<String>> {
        let next = input.bytes.saturating_add(self.bytes);
        let cost = next.div_ceil(64) - input.bytes.div_ceil(64);
        charge(&mut *input.store, cost)?;
        if next > 1_048_576 {
            return Ok(None);
        }
        input.bytes = next;
        // All transcoding and the final owned copy happen after the bound and debit.
        Ok(Some(self.cursor.to_str(&*input.store)?.into_owned()))
    }
}

/// The outer argument list is bounded by a trusted thunk signature and does
/// not represent value cells. Nested value lists retain their own budgets.
pub(super) struct Arguments<T>(pub(super) wasmtime::component::WasmList<T>);

fn read_list<T: Materialize + Lift + 'static>(
    values: wasmtime::component::WasmList<T>,
    input: &mut Input<'_, '_>,
    meter_cells: bool,
) -> wasmtime::Result<Option<Vec<T::Owned>>> {
    if meter_cells {
        charge(&mut *input.store, values.len())?;
    }
    if values.len() > 65_536 {
        return Ok(None);
    }
    let parent_bytes = std::mem::replace(&mut input.bytes, 0);
    let mut owned = Vec::new();
    for index in 0..values.len() {
        let value = values.get(&mut *input.store, index).unwrap()?;
        let Some(value) = value.materialize(input)? else {
            input.bytes = parent_bytes;
            return Ok(None);
        };
        owned.push(value);
    }
    input.bytes = parent_bytes;
    Ok(Some(owned))
}

impl<T: Materialize + Lift + 'static> Materialize for wasmtime::component::WasmList<T> {
    type Owned = Vec<T::Owned>;
    fn materialize(self, input: &mut Input<'_, '_>) -> wasmtime::Result<Option<Self::Owned>> {
        read_list(self, input, true)
    }
}

impl<T: Materialize + Lift + 'static> Materialize for Arguments<T> {
    type Owned = Vec<T::Owned>;
    fn materialize(self, input: &mut Input<'_, '_>) -> wasmtime::Result<Option<Self::Owned>> {
        read_list(self.0, input, false)
    }
}

macro_rules! scalar {
    ($($ty:ty),* $(,)?) => { $(impl Materialize for $ty {
        type Owned = Self;
        fn materialize(self, _: &mut Input<'_, '_>) -> wasmtime::Result<Option<Self>> { Ok(Some(self)) }
    })* };
}
scalar!(bool, char, i64, u32, u64, f64);

impl Materialize for Resource<wire::Node> {
    type Owned = Self;
    fn materialize(self, input: &mut Input<'_, '_>) -> wasmtime::Result<Option<Self>> {
        charge(&mut *input.store, node_cost(&self))?;
        Ok(Some(self))
    }
}

// Direct host tests already own their cells; production registrations accept
// only the generated cursor types. Keep the control route's original debit.
#[cfg(test)]
impl Materialize for Vec<wire::Cell> {
    type Owned = Self;
    fn materialize(self, input: &mut Input<'_, '_>) -> wasmtime::Result<Option<Self>> {
        charge(&mut *input.store, wire_cost(&self))?;
        Ok(Some(self))
    }
}

impl<A: Materialize, B: Materialize> Materialize for (A, B) {
    type Owned = (A::Owned, B::Owned);
    fn materialize(self, input: &mut Input<'_, '_>) -> wasmtime::Result<Option<Self::Owned>> {
        let Some(a) = self.0.materialize(input)? else {
            return Ok(None);
        };
        let Some(b) = self.1.materialize(input)? else {
            return Ok(None);
        };
        Ok(Some((a, b)))
    }
}

pub(super) fn import<V: Materialize, T>(
    mut store: wasmtime::StoreContextMut<'_, State>,
    base: usize,
    validate: impl FnOnce() -> Result<(), wire::Refusal>,
    value: V,
    apply: impl FnOnce(V::Owned) -> Result<T, wire::Refusal>,
    output: impl FnOnce(&T) -> usize,
) -> wasmtime::Result<Result<T, wire::Refusal>> {
    contain(|| {
        #[cfg(test)]
        let before = store.get_fuel().ok();
        charge(&mut store, 1 + base)?;
        let mut input = Input::new(&mut store);
        let result = match validate() {
            Err(error) => Err(error),
            Ok(()) => match value.materialize(&mut input)? {
                Some(value) => apply(value),
                None => Err(wire::Refusal::NotHere("internals budget".into())),
            },
        };
        charge(&mut store, result.as_ref().map_or(0, output))?;
        #[cfg(test)]
        if let Some(before) = before {
            super::tests::FUEL_DEBITS
                .with_borrow_mut(|debits| debits.push(before - store.get_fuel().unwrap()));
        }
        Ok(result)
    })
}

include!(concat!(env!("OUT_DIR"), "/internals_lazy.rs"));

// SAFETY: this is exactly WasmStr's canonical representation and type check.
unsafe impl ComponentType for StringInput {
    type Lower = <WasmStr as ComponentType>::Lower;
    const ABI: CanonicalAbiInfo = WasmStr::ABI;

    fn typecheck(ty: &InterfaceType, types: &InstanceType<'_>) -> wasmtime::Result<()> {
        WasmStr::typecheck(ty, types)
    }
}

// SAFETY: WasmStr performs all canonical pointer, alignment and bounds validation.
// We only inspect that validated range to measure UTF-8 size without allocating.
unsafe impl Lift for StringInput {
    fn linear_lift_from_flat(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        src: &Self::Lower,
    ) -> wasmtime::Result<Self> {
        let cursor = WasmStr::linear_lift_from_flat(cx, ty, src)?;
        Self::measured(cx, cursor, src[0].get_u32(), src[1].get_u32())
    }

    fn linear_lift_from_memory(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        bytes: &[u8],
    ) -> wasmtime::Result<Self> {
        let cursor = WasmStr::linear_lift_from_memory(cx, ty, bytes)?;
        let ptr = u32::from_le_bytes(bytes[..4].try_into()?);
        let len = u32::from_le_bytes(bytes[4..8].try_into()?);
        Self::measured(cx, cursor, ptr, len)
    }
}

impl StringInput {
    fn measured(
        cx: &LiftContext<'_>,
        cursor: WasmStr,
        ptr: u32,
        len: u32,
    ) -> wasmtime::Result<Self> {
        let bytes = utf8_size(cx.memory(), ptr as usize, len, cx.options().string_encoding)?;
        Ok(Self { cursor, bytes })
    }
}

/// Count decoded bytes without making the UTF-16/Latin-1 temporary that to_str
/// may allocate. Pointer pairs follow Wasmtime 50's memory32 canonical ABI.
fn utf8_size(
    memory: &[u8],
    ptr: usize,
    len: u32,
    encoding: StringEncoding,
) -> wasmtime::Result<usize> {
    let tagged = len & (1 << 31) != 0;
    let utf16 =
        encoding == StringEncoding::Utf16 || (encoding == StringEncoding::CompactUtf16 && tagged);
    let units = if encoding == StringEncoding::CompactUtf16 && tagged {
        len ^ (1 << 31)
    } else {
        len
    } as usize;
    let size = units
        .checked_mul(if utf16 { 2 } else { 1 })
        .ok_or_else(|| wasmtime::Error::msg("string length overflow"))?;
    let end = ptr
        .checked_add(size)
        .ok_or_else(|| wasmtime::Error::msg("string range overflow"))?;
    let bytes = memory
        .get(ptr..end)
        .ok_or_else(|| wasmtime::Error::msg("string out of bounds"))?;
    if utf16 {
        char::decode_utf16(
            bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes([b[0], b[1]])),
        )
        .try_fold(0usize, |size, c| Ok(size + c?.len_utf8()))
    } else if encoding == StringEncoding::Utf8 {
        Ok(std::str::from_utf8(bytes)?.len())
    } else {
        Ok(bytes.iter().map(|b| if *b < 128 { 1 } else { 2 }).sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_decoded_bytes_for_every_canonical_encoding() {
        assert_eq!(
            utf8_size("aé😀".as_bytes(), 0, 7, StringEncoding::Utf8).unwrap(),
            7
        );
        let utf16: Vec<_> = "aé😀".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(utf8_size(&utf16, 0, 4, StringEncoding::Utf16).unwrap(), 7);
        assert_eq!(
            utf8_size(&utf16, 0, 4 | (1 << 31), StringEncoding::CompactUtf16).unwrap(),
            7
        );
        assert_eq!(
            utf8_size(&[b'a', 233], 0, 2, StringEncoding::CompactUtf16).unwrap(),
            3
        );
        assert!(utf8_size(&[0, 216], 0, 1, StringEncoding::Utf16).is_err());
        assert!(utf8_size(&[255], 0, 1, StringEncoding::Utf8).is_err());
        assert!(utf8_size(&[], 0, 1, StringEncoding::Utf16).is_err());
    }
}
