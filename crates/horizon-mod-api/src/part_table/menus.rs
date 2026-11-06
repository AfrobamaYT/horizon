impl HookParts {
    pub(crate) fn menu<'k>(
        &self,
        id: impl FnOnce() -> Option<&'k str>,
    ) -> &[Attached<dyn MenuPart>] {
        attached(&self.menus, id)
    }
}

/// What a mod's [`Hooks::register`](super::Hooks::register) attaches its parts with, and
/// declares its block entity types and ticket controllers on. A thing takes one part of each
/// kind: the same kind twice on one thing refuses the mods at startup, naming both.
///
/// Mods call the generic methods on `dyn Registrar` after requesting registry keys;
/// they reach the trait's own methods, one implementation for native and wasm mods.
pub trait Registrar {
    /// Attach `part`, of kind `kind`, to the mod menu type `menu`.
    fn attach_menu_part(&mut self, menu: &str, kind: &'static str, part: Arc<dyn MenuPart>);
}

impl dyn Registrar + '_ {
    /// Attach `part` to the mod menu type `menu`.
    pub fn menu_part<P: MenuPart>(&mut self, menu: &str, part: P) {
        self.attach_menu_part(menu, type_name::<P>(), Arc::new(part));
    }
}

impl Registrar for PartRegistrar {
    fn attach_menu_part(&mut self, menu: &str, kind: &'static str, part: Arc<dyn MenuPart>) {
        self.attach(|p| &mut p.menus, kind, menu, part);
    }
}
