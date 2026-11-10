use super::*;
#[test]
fn registration_refuses_unknown_books_duplicate_menus_and_overlapping_roles() {
    let mut registrar = PartRegistrar::new("demo");
    let book = registrar.recipe_book_type("demo:book").unwrap();
    let valid = crate::RecipeBookLayout::Furnace {
        input: 0,
        fuel: 1,
        result: 2,
    };
    assert!(
        registrar
            .menu_recipe_book("demo:pot", &book, "demo:cooking", valid.clone())
            .is_ok()
    );
    assert!(
        registrar
            .menu_recipe_book("demo:pot", &book, "demo:cooking", valid.clone())
            .is_err()
    );
    assert!(
        registrar
            .menu_recipe_book("other:pot", &book, "demo:cooking", valid)
            .is_err()
    );
    assert!(
        registrar
            .menu_recipe_book(
                "demo:bad",
                &book,
                "demo:cooking",
                crate::RecipeBookLayout::Stonecutter {
                    input: 0,
                    result: 0
                }
            )
            .is_err()
    );
    let unknown = mod_vocab::__private::parts_installation::recipe_book_type("demo:unknown".into());
    assert!(
        registrar
            .menu_recipe_book(
                "demo:unknown",
                &unknown,
                "demo:cooking",
                crate::RecipeBookLayout::Stonecutter {
                    input: 0,
                    result: 1
                }
            )
            .is_err()
    );
    assert_eq!(registrar.menu_recipe_books.len(), 1);
    assert_eq!(registrar.refused.len(), 4);
}
