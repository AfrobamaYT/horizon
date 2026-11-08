//! Sparse updates to ordinary Wire fields. The schema owns field ids and native destinations;
//! no engine type, address or layout is part of a patch.
use super::{Reader, Record, Wire, WireError, Writer};

type CellBytes = (u32, Vec<u8>);
const MAX_CELLS: usize = 65_536;
const MAX_VALUES: usize = 1_048_576;
const MAX_PACKET: usize = 4 + MAX_CELLS * <CellBytes as Wire>::MIN + MAX_VALUES;

/// A sparse field update record. Entries are unique and sorted by their schema's field ids.
/// Each value uses its ordinary Wire codec; the receiving schema checks its concrete type.
#[derive(Default)]
pub struct RecordPatch {
    cells: Vec<CellBytes>,
    bytes: usize,
}
impl RecordPatch {
    /// Add a pre-encoded ordinary Wire value, preserving the patch if its limits are exceeded.
    /// The receiving schema validates its type, field id and write permission.
    pub fn set_encoded(&mut self, field: u32, value: &[u8]) -> Result<(), WireError> {
        let position = self.cells.binary_search_by_key(&field, |cell| cell.0);
        if position.is_err() && self.cells.len() == MAX_CELLS {
            return Err(WireError::Length);
        }
        let previous = position.as_ref().ok().map_or(0, |&i| self.cells[i].1.len());
        let remaining = MAX_VALUES - (self.bytes - previous);
        if value.len() > remaining {
            return Err(WireError::Length);
        }
        let value = value.to_vec();
        self.bytes = self.bytes - previous + value.len();
        match position {
            Ok(i) => self.cells[i].1 = value,
            Err(i) => self.cells.insert(i, (field, value)),
        }
        Ok(())
    }

    /// Add or replace a field's encoded value, retaining the previous patch on refusal.
    /// The receiver, not the sender, owns the field id and write permission checks.
    pub fn set<T: Wire>(&mut self, field: u32, value: &T) -> Result<(), WireError> {
        let position = self.cells.binary_search_by_key(&field, |cell| cell.0);
        if position.is_err() && self.cells.len() == MAX_CELLS {
            return Err(WireError::Length);
        }
        let previous = position.as_ref().ok().map_or(0, |&i| self.cells[i].1.len());
        let remaining = MAX_VALUES - (self.bytes - previous);
        let mut out = Writer::new();
        out.limit(remaining);
        out.put(value);
        if let Some(error) = out.error() {
            return Err(error);
        }
        let bytes = out.into_bytes();
        self.bytes = self.bytes - previous + bytes.len();
        match position {
            Ok(i) => self.cells[i].1 = bytes,
            Err(i) => self.cells.insert(i, (field, bytes)),
        }
        Ok(())
    }
}
impl Wire for RecordPatch {
    const MIN: usize = <Vec<CellBytes> as Wire>::MIN;
    fn put(&self, out: &mut Writer) {
        out.put(&self.cells);
    }
    fn take(input: &mut Reader<'_>) -> Result<Self, WireError> {
        input.bounded(MAX_PACKET, |input| {
            let mut remaining = MAX_VALUES;
            let mut previous = None;
            let cells = input.list_with(MAX_CELLS, |input| {
                let cell: CellBytes =
                    input.bounded(remaining + CellBytes::MIN, |input| input.take())?;
                if previous.is_some_and(|previous| previous >= cell.0) {
                    return Err(WireError::Invalid(
                        "patch fields are not unique and ordered",
                    ));
                }
                previous = Some(cell.0);
                remaining -= cell.1.len();
                Ok(cell)
            })?;
            Ok(Self {
                cells,
                bytes: MAX_VALUES - remaining,
            })
        })
    }
}

/// A native destination paired with its schema's field id. The borrow remains local to the call.
pub struct PatchCell<'a, T> {
    field: u32,
    value: &'a mut T,
}
impl<'a, T> PatchCell<'a, T> {
    /// Bind a native field to the id emitted by its schema.
    pub fn new(field: u32, value: &'a mut T) -> Self {
        Self { field, value }
    }
}

/// The checked patch input used by generated field bindings.
#[doc(hidden)]
pub struct PatchInput<'a> {
    patch: &'a RecordPatch,
    used: usize,
}
impl PatchInput<'_> {
    fn take<T: Wire>(&mut self, field: u32) -> Result<Option<T>, WireError> {
        match self.patch.cells.binary_search_by_key(&field, |cell| cell.0) {
            Ok(i) => {
                let value = Reader::decode(&self.patch.cells[i].1)?;
                self.used += 1;
                Ok(Some(value))
            }
            Err(_) => Ok(None),
        }
    }
}

/// A generated field list's typed plan and commit operations.
#[doc(hidden)]
pub trait PatchSet {
    type Plan;
    fn plan(&self, input: &mut PatchInput<'_>) -> Result<Self::Plan, WireError>;
    fn commit(&mut self, plan: Self::Plan);
}
impl PatchSet for () {
    type Plan = ();
    fn plan(&self, _: &mut PatchInput<'_>) -> Result<(), WireError> {
        Ok(())
    }
    fn commit(&mut self, _: ()) {}
}
impl<H: Wire, T: PatchSet> PatchSet for (PatchCell<'_, H>, T) {
    type Plan = (Option<H>, T::Plan);
    fn plan(&self, input: &mut PatchInput<'_>) -> Result<Self::Plan, WireError> {
        Ok((input.take(self.0.field)?, self.1.plan(input)?))
    }
    fn commit(&mut self, (head, tail): Self::Plan) {
        if let Some(value) = head {
            *self.0.value = value;
        }
        self.1.commit(tail);
    }
}

/// A sparse-update hook record backed by typed native fields, without a second set of DTOs.
/// Decoding and planning finish before any destination changes. Unbound, readonly, malformed
/// and trailing cells refuse the whole patch. An empty patch leaves every destination untouched.
pub struct PatchFields<T> {
    fields: T,
    nodes: u32,
}
impl<T: PatchSet> PatchFields<T> {
    /// Bind a generated field list and its id domain. Every writable id occurs once in the list.
    pub fn new(fields: T, nodes: u32) -> Self {
        Self { fields, nodes }
    }
}
impl<T: PatchSet> Record for PatchFields<T> {
    fn encode(&self, out: &mut Writer) {
        out.put(&RecordPatch::default());
    }
    fn apply(&mut self, bytes: &[u8]) -> Result<(), WireError> {
        let patch = Reader::decode::<RecordPatch>(bytes)?;
        if patch.cells.last().is_some_and(|cell| cell.0 >= self.nodes) {
            return Err(WireError::Invalid("patch field is outside the schema"));
        }
        let mut input = PatchInput {
            patch: &patch,
            used: 0,
        };
        let plan = self.fields.plan(&mut input)?;
        if input.used != patch.cells.len() {
            return Err(WireError::Invalid("patch field is not writable"));
        }
        self.fields.commit(plan);
        Ok(())
    }
}
