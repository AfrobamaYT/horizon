//! Guest listeners sharing the ordinary hook export and wire arena.
use crate::{
    PointId,
    registry::{ApiKind, SlotListener},
};
use std::cell::RefCell;

thread_local! {
    static LISTENERS: RefCell<Vec<(usize, *const ())>> = const { RefCell::new(Vec::new()) };
}

/// Installs a typed API listener in this guest and enables the generated export table.
/// The host registers the subscribed places in its ordinary ordered listener list.
pub fn register(listener: &SlotListener) -> Result<(), crate::RegisterError> {
    if let Some(ApiKind::Typed(function)) = listener.api_kind() {
        LISTENERS.with(|slots| slots.borrow_mut().push((listener.place(), function)));
        mod_api::wire::guest::extend(0x8000_0000, 0xC000_0000, crate::guest_dispatch);
        Ok(())
    } else {
        Err(crate::RegisterError::Unsupported)
    }
}

/// The places this guest listens at, in registration order.
pub fn subscriptions() -> Vec<u32> {
    LISTENERS.with(|slots| {
        slots
            .borrow()
            .iter()
            .map(|(place, _)| *place as u32)
            .collect()
    })
}

pub(crate) fn fire(place: PointId, mut call: impl FnMut(*const ())) {
    // Copy one entry at a time; listeners may reenter the guest through the host.
    let len = LISTENERS.with(|slots| slots.borrow().len());
    for index in 0..len {
        let (at, function) = LISTENERS.with(|slots| slots.borrow()[index]);
        if at == place as usize {
            call(function);
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn horizon_layout_ext() -> u64 {
    crate::POINT_LAYOUT
}
