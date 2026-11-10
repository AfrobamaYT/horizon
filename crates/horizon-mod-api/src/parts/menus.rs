//! Concrete authoring extracted verbatim from the vocabulary layer.
use super::*;

impl MenusRegistrar for PartRegistrar {
    fn attach_menu_part(&mut self, menu: &str, kind: &'static str, part: Arc<dyn MenuPart>) {
        self.attach(|p| &mut p.menus, kind, menu, part);
    }
}

