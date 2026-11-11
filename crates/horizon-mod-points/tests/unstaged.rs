//! Plain builds must not accept listeners that can never fire.
use mod_points::{PointId, RegisterError};

#[test]
fn native_listener_is_refused_by_mod_and_target() {
    if mod_points::STAGED {
        return;
    }
    let slot = mod_points::listeners::tick("named_mod", 0, "head", tick);
    let error = mod_points::register(&slot).unwrap_err();
    assert!(matches!(error, RegisterError::Unstaged { .. }));
    let message = error.to_string();
    assert!(
        message.contains("named_mod")
            && message.contains("::tick")
            && message.contains("no point sites"),
        "{message}"
    );
    // SAFETY: this callback has the erased signature; refusal means it is never invoked.
    let raw = unsafe {
        mod_points::__private::SlotListener::erased("native_raw", PointId::Tick.place(), erased)
    };
    let message = mod_points::register(&raw).unwrap_err().to_string();
    assert!(
        message.contains("native_raw") && message.contains("::tick"),
        "{message}"
    );
}
fn tick(_: mod_api::HostSide<'_>, _: &mut mod_points::TickArgs) {}

unsafe fn erased(_: usize, _: *mut (), _: *mut ()) {}
