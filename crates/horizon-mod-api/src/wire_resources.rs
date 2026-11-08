//! A call's resource registry. Data still uses Wire; only trait objects need the active side.
use super::{Id, WireError};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    rc::Rc,
};

/// Resolves resource identities for the side of a synchronous boundary call.
pub trait ResourceCodec {
    fn store(&self, value: Box<dyn Any>) -> Result<Id, WireError>;
    fn load(&self, id: Id, view: TypeId) -> Result<Box<dyn Any>, WireError>;
}
thread_local! { static ACTIVE: RefCell<Option<Rc<dyn ResourceCodec>>> = const { RefCell::new(None) }; }
/// Restores the outer call's codec on return, including guest-to-host-to-guest callbacks.
pub struct ResourceScope {
    previous: Option<Rc<dyn ResourceCodec>>,
}
impl ResourceScope {
    pub fn new(codec: Rc<dyn ResourceCodec>) -> Self {
        Self {
            previous: ACTIVE.with(|active| active.replace(Some(codec))),
        }
    }
}
impl Drop for ResourceScope {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.replace(self.previous.take()));
    }
}
pub(super) fn store(value: Box<dyn Any>) -> Result<Id, WireError> {
    let codec = ACTIVE
        .with(|active| active.borrow().clone())
        .ok_or(WireError::Invalid("no resource context"))?;
    codec.store(value)
}
pub(super) fn load<T: 'static>(id: Id) -> Result<T, WireError> {
    let codec = ACTIVE
        .with(|active| active.borrow().clone())
        .ok_or(WireError::Invalid("no resource context"))?;
    codec
        .load(id, TypeId::of::<T>())?
        .downcast::<T>()
        .map(|value| *value)
        .map_err(|_| WireError::Invalid("wrong resource view"))
}
