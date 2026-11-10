//! A custom menu has no template-owned spare space. Use external gutters unless
//! its declaration explicitly reserves an internal toggle position.
#[derive(Debug, PartialEq)]
pub(super) struct BookLayout {
    pub shift: f32,
    pub overlay: bool,
    pub toggle: (f32, f32),
}
pub(super) fn layout(
    view: [f32; 2],
    menu: [f32; 2],
    anchor: Option<[f32; 2]>,
    open: bool,
) -> Option<BookLayout> {
    // Vanilla's fixed 77-pixel translation only separates its 176-wide menu.
    let overlay = menu[0] > 176.0 || view[0] < 379.0 || view[0] < menu[0] + 202.0;
    let shift = if open && !overlay { 77.0 } else { 0.0 };
    let x = ((view[0] - menu[0]) / 2.0).trunc() + shift;
    let y = ((view[1] - menu[1]) / 2.0).trunc();
    let fits =
        |bx: f32, by: f32| bx >= 0.0 && by >= 0.0 && bx + 20.0 <= view[0] && by + 18.0 <= view[1];
    let toggle = if let Some([ax, ay]) = anchor {
        let pos = (x + ax, y + ay);
        fits(pos.0, pos.1).then_some(pos)?
    } else {
        [
            (x + menu[0] + 4.0, y + 5.0),
            (x - 24.0, y + 5.0),
            (x + menu[0] - 20.0, y - 22.0),
        ]
        .into_iter()
        .find(|&(bx, by)| fits(bx, by))?
    };
    Some(BookLayout {
        shift,
        overlay,
        toggle,
    })
}
pub(super) fn refusal(block: &str, menu: &str) -> String {
    format!(
        "recipe book refused for mod block {block} menu {menu}: no usable toggle gutter; declare Screen.recipe_book_toggle"
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_gutter_overlay_and_anchor_positions() {
        assert_eq!(
            layout([427.0, 240.0], [176.0, 166.0], None, true),
            Some(BookLayout {
                shift: 77.0,
                overlay: false,
                toggle: (382.0, 42.0)
            })
        );
        assert_eq!(
            layout([427.0, 240.0], [300.0, 166.0], None, true),
            Some(BookLayout {
                shift: 0.0,
                overlay: true,
                toggle: (367.0, 42.0)
            })
        );
        assert_eq!(
            layout([240.0, 240.0], [220.0, 166.0], None, false),
            Some(BookLayout {
                shift: 0.0,
                overlay: true,
                toggle: (210.0, 15.0)
            })
        );
        assert_eq!(
            layout([240.0, 240.0], [240.0, 240.0], Some([8.0, 6.0]), true),
            Some(BookLayout {
                shift: 0.0,
                overlay: true,
                toggle: (8.0, 6.0)
            })
        );
        assert_eq!(
            layout([600.0, 240.0], [300.0, 166.0], None, true),
            Some(BookLayout {
                shift: 0.0,
                overlay: true,
                toggle: (454.0, 42.0)
            })
        );
        assert!(layout([240.0, 240.0], [240.0, 240.0], None, false).is_none());
        assert_eq!(
            refusal("demo:machine", "demo:menu"),
            "recipe book refused for mod block demo:machine menu demo:menu: no usable toggle gutter; declare Screen.recipe_book_toggle"
        );
        assert!(layout([240.0, 240.0], [240.0, 240.0], Some([f32::NAN, 2.0]), false).is_none());
    }
}
