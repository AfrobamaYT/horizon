//! Validate registered destination roles before publishing content menu types.
use horizon_client::recipe_book::{
    menu_layout::{MenuLayoutMetadata, compile},
    slot_layout::LayoutError,
};
use horizon_mod_package::content::MenuType;
use mod_api::RecipeBookLayout;

pub(super) fn validate_registered(menus: &[&MenuType]) -> Result<(), String> {
    for menu in RecipeBookLayout::registered_menus() {
        let Some((_, _, layout)) = RecipeBookLayout::registered(&menu) else {
            continue;
        };
        validate(menus, &menu, layout)?;
    }
    Ok(())
}

fn validate(menus: &[&MenuType], id: &str, layout: RecipeBookLayout) -> Result<(), String> {
    let menu = menus
        .iter()
        .find(|menu| menu.id == id)
        .ok_or_else(|| format!("recipe book menu {id:?}: unknown content menu"))?;
    let metadata = MenuLayoutMetadata {
        slots: menu.slots,
        crafting: menu
            .crafting
            .map(|grid| (grid.grid, grid.size, grid.result)),
    };
    compile(layout, metadata)
        .map_err(|error: LayoutError| format!("recipe book menu {id:?}: {error:?}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn menu() -> MenuType {
        MenuType {
            id: "demo:pot".into(),
            slots: 3,
            data_slots: 0,
            results: vec![2],
            crafting: None,
            links: None,
        }
    }
    #[test]
    fn actual_content_count_is_required_before_publication() {
        let menu = menu();
        let roles = RecipeBookLayout::Furnace {
            input: 0,
            fuel: 1,
            result: 2,
        };
        assert!(validate(&[&menu], "demo:pot", roles.clone()).is_ok());
        assert!(
            validate(&[], "demo:pot", roles.clone())
                .unwrap_err()
                .contains("unknown content menu")
        );
        let short = MenuType { slots: 2, ..menu };
        assert!(
            validate(&[&short], "demo:pot", roles)
                .unwrap_err()
                .contains("OutOfRange(2)")
        );
    }
    #[test]
    fn a_crafting_map_cannot_invent_crafting_storage() {
        let menu = menu();
        let roles = RecipeBookLayout::Crafting {
            width: 1,
            height: 1,
            slots: vec![0],
            result: 2,
        };
        assert!(
            validate(&[&menu], "demo:pot", roles)
                .unwrap_err()
                .contains("InvalidGrid")
        );
    }
}
