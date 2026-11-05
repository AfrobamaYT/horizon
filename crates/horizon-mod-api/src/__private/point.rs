//! Injection points (#589): one slot per point's head and return, in the game's `POINTS` table,
//! set once at load where an installed mod listens. An empty slot costs one atomic load and a
//! branch; the listeners run out of line. A mod never names this module.
/// The mods listening at `point`, in dispatch order, each marked as running `name` so a panic in
/// it names the mod. The game's `#[point]` code calls it only while its slot says one listens.
pub fn listeners(
    point: crate::points::PointId,
    name: &'static str,
) -> impl Iterator<Item = &'static dyn crate::points::Points> {
    crate::entered_points(point, name)
}

/// Whether an installed mod listens at the point with this index: the game sets its slot table
/// from it once, after the mods are installed.
pub fn listened_at(index: usize) -> bool {
    crate::point_listened_at(index)
}

/// Whether `list` holds `name`, at compile time: the launcher's generated `main` checks each
/// point injection's method against the mod's `__HORIZON_POINTS` with it.
pub const fn lists(list: &[&str], name: &str) -> bool {
    let mut i = 0;
    while i < list.len() {
        let (a, b) = (list[i].as_bytes(), name.as_bytes());
        if a.len() == b.len() {
            let mut j = 0;
            while j < a.len() && a[j] == b[j] {
                j += 1;
            }
            if j == a.len() {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Runs a point's listeners, out of the pointed function's own code.
#[cold]
#[inline(never)]
pub fn fire(listeners: impl FnOnce()) {
    listeners();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_finds_a_name_at_compile_time() {
        const LIST: &[&str] = &["tick", "tick_return"];
        const _: () = assert!(lists(LIST, "tick_return"));
        assert!(!lists(LIST, "tic") && !lists(LIST, "ticks") && !lists(&[], "tick"));
    }
}
