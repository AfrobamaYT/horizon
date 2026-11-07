use super::*;
/// The ticket controllers registered at startup.
pub(crate) fn installed_controllers() -> &'static [ControllerInfo] {
    INSTALLED
        .get()
        .map_or(&[], |installed| installed.controllers.as_slice())
}

/// The ticket controllers `mods` register, for a test's mods.
#[cfg(any(test, feature = "testing"))]
pub(crate) fn registered_controllers(mods: &[ModEntry]) -> Vec<ControllerInfo> {
    if mods.is_empty() {
        return Vec::new();
    }
    register_all(mods).controllers
}
/// Fixtures that tests of other crates use to install Parts.
#[cfg(any(test, feature = "testing"))]
pub mod tests {
    use super::*;

    #[test]
    pub(crate) fn ticket_controllers_get_the_callers_namespace_and_refuse_anothers() {
        let mut r = super::PartRegistrar::new("first");
        r.register_ticket_controller("quarry", true);
        r.register_ticket_controller("first:drill", false);
        assert!(r.refused.is_empty(), "{:?}", r.refused);
        let ids: Vec<_> = r
            .controllers
            .iter()
            .map(|c| (&*c.id, c.validates))
            .collect();
        assert_eq!(ids, [("first:quarry", true), ("first:drill", false)]);
        // Another mod's namespace, and no name at all.
        r.register_ticket_controller("second:quarry", true);
        r.register_ticket_controller("first:", true);
        r.register_ticket_controller("", true);
        assert_eq!(r.refused.len(), 3, "{:?}", r.refused);
        assert!(r.refused[0].contains("first registered ticket controller \"second:quarry\""));
        assert!(r.refused[0].contains("not its own"));
        assert!(r.refused[1].contains("no name") && r.refused[2].contains("no name"));
        assert_eq!(r.controllers.len(), 2);
    }

    #[test]
    pub(crate) fn a_ticket_controller_registered_twice_refuses_the_mods_naming_mod_and_id() {
        let mut r = super::PartRegistrar::new("first");
        r.register_ticket_controller("quarry", true);
        r.register_ticket_controller("first:quarry", false);
        r.mod_id = "second";
        // The same path under another mod is another id.
        r.register_ticket_controller("quarry", true);
        assert_eq!(r.refused.len(), 1, "{:?}", r.refused);
        assert_eq!(
            r.refused[0],
            "first registered ticket controller first:quarry twice"
        );
        assert_eq!(r.controllers.len(), 2);
        assert_eq!(&*r.controllers[1].id, "second:quarry");
    }
}
