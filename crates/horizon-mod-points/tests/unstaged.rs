//! Plain builds must not accept listeners that can never fire.
use mod_points::RegisterError;

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
}
fn tick(_: mod_api::HostSide<'_>, _: &mut mod_points::TickArgs) {}
