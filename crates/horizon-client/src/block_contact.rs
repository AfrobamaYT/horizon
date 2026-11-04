//! Synchronous mod-block contacts from the server's simulation code.
use horizon_core::position::BlockPos;
use horizon_world::registry::BlockStateId;
use std::{
    cell::{Cell, RefCell},
    sync::OnceLock,
};

#[derive(Clone, Copy, Debug)]
pub struct Contact<'a> {
    pub entity: i32,
    pub kind: &'a str,
    pub block: &'a str,
    pub state: BlockStateId,
    pub position: BlockPos,
}

#[derive(Clone, Copy, Debug)]
pub enum Event<'a> {
    StepOn(Contact<'a>),
    FallOn(Contact<'a>, f64),
}

/// An entity inside a block whose Part listens to `entity_inside`
/// (`Block.entityInside`), kept until the entity's batch ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inside {
    pub entity: i32,
    pub state: BlockStateId,
    pub position: BlockPos,
    /// `Entity.checkInsideBlocks`' `isPrecise`: the entity moved far, or its box
    /// at the target intersects the block.
    pub precise: bool,
}

pub static HOOK: OnceLock<fn(&str, Event<'_>, &mut f32)> = OnceLock::new();
/// Whether any part listens to `entity_inside`, and whether one does for a block
/// state; the game sets them from the part table.
pub static ANY_INSIDE: OnceLock<fn() -> bool> = OnceLock::new();
pub static INSIDE_LISTENS: OnceLock<fn(BlockStateId) -> bool> = OnceLock::new();
thread_local! {
    static LEVEL: Cell<Option<&'static str>> = const { Cell::new(None) };
    static PENDING: RefCell<Vec<Inside>> = const { RefCell::new(Vec::new()) };
}

/// Scope authority to this thread and restore it even on an error or panic.
/// Nested guest actions may temporarily enter a different dimension.
pub fn in_level<R>(dimension: Option<&'static str>, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<&'static str>);
    impl Drop for Restore {
        fn drop(&mut self) {
            LEVEL.set(self.0);
        }
    }
    let _restore = Restore(LEVEL.replace(dimension));
    f()
}

pub fn emit(event: Event<'_>) {
    dispatch(event, &mut 0.0);
}

/// Damage is the calculated fall damage, before hurt handling and fall sounds.
pub fn fall(contact: Contact<'_>, distance: f64, mut damage: f32) -> f32 {
    dispatch(Event::FallOn(contact, distance), &mut damage);
    damage
}

/// Note an entity inside a block, if a part there listens; nothing is kept
/// otherwise, so a world without such a part pays one test per block.
pub fn inside(entity: i32, state: BlockStateId, position: BlockPos, precise: bool) {
    if LEVEL.get().is_some() && INSIDE_LISTENS.get().is_some_and(|listens| listens(state)) {
        PENDING.with_borrow_mut(|pending| {
            pending.push(Inside {
                entity,
                state,
                position,
                precise,
            });
        });
    }
}

/// The contacts noted since the last call, in the order they happened.
pub fn take_inside() -> Vec<Inside> {
    PENDING.with_borrow_mut(std::mem::take)
}

/// `Entity.checkInsideBlocks`' `isPrecise` for the block at `position`: the
/// move from `from` to `to` was long, or the box at `to` touches the block.
pub fn is_precise(
    from: horizon_core::geometry::Vec3,
    to: horizon_core::geometry::Vec3,
    at_target: horizon_core::geometry::Aabb,
    position: BlockPos,
) -> bool {
    use horizon_core::geometry::{Aabb, Vec3};
    let far = 0.999_990_000_000_252_6_f64;
    let min = Vec3::new(
        f64::from(position.x),
        f64::from(position.y),
        f64::from(position.z),
    );
    from.distance_squared(to) > far * far
        || at_target.intersects(Aabb::new(min, min.add_vector(Vec3::new(1.0, 1.0, 1.0))))
}

fn dispatch(event: Event<'_>, damage: &mut f32) {
    let contact = match event {
        Event::StepOn(c) | Event::FallOn(c, _) => c,
    };
    if contact.block.starts_with("minecraft:") {
        return;
    }
    if let Some(dimension) = LEVEL.get()
        && let Some(hook) = HOOK.get()
    {
        let original = *damage;
        hook(dimension, event, damage);
        if !damage.is_finite() {
            *damage = original;
        }
        *damage = damage.max(0.0);
    }
}

/// Reuse the entity block sweep for item and projectile ticks without an effect collector.
pub fn swept_inside(
    entity: i32,
    from: horizon_core::geometry::Vec3,
    to: horizon_core::geometry::Vec3,
    bounds: horizon_core::geometry::Aabb,
    mut block: impl FnMut(BlockPos) -> BlockStateId,
) {
    if LEVEL.get().is_none() || !ANY_INSIDE.get().is_some_and(|any| any()) {
        return;
    }
    let bounds = bounds.deflate(horizon_core::geometry::Vec3::new(1e-5, 1e-5, 1e-5));
    let mut visited = std::collections::HashSet::new();
    let _ =
        horizon_core::geometry::sweep_intersected_blocks(from, to, bounds, 1_000_000, |p, step| {
            if step >= 16 {
                return false;
            }
            if visited.insert(p) {
                inside(entity, block(p), p, is_precise(from, to, bounds, p));
            }
            true
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    thread_local! {
        static SEEN: Cell<usize> = const { Cell::new(0) };
    }
    fn hook(dimension: &str, event: Event<'_>, damage: &mut f32) {
        assert_eq!(dimension, "minecraft:the_nether");
        let c = match event {
            Event::FallOn(c, distance) => {
                assert_eq!(distance, 12.0);
                *damage = match *damage {
                    99.0 => f32::NAN,
                    98.0 => -1.0,
                    _ => *damage * 0.2,
                };
                c
            }
            Event::StepOn(c) => c,
        };
        assert_eq!(
            (c.entity, c.kind, c.state, c.position),
            (
                17,
                "minecraft:zombie",
                BlockStateId(9),
                BlockPos::new(1, 2, 3)
            )
        );
        SEEN.set(SEEN.get() + 1);
    }
    #[test]
    fn contacts_require_server_scope_and_mod_blocks_and_restore_nested_levels() {
        HOOK.set(hook).unwrap();
        let c = Contact {
            entity: 17,
            kind: "minecraft:zombie",
            block: "test:hay",
            state: BlockStateId(9),
            position: BlockPos::new(1, 2, 3),
        };
        assert_eq!(fall(c, 12.0, 10.0), 10.0);
        in_level(Some("minecraft:the_nether"), || {
            emit(Event::StepOn(c));
            assert_eq!(fall(c, 12.0, 10.0), 2.0);
            in_level(None, || assert_eq!(fall(c, 12.0, 10.0), 10.0));
            assert_eq!(fall(c, 12.0, 0.0), 0.0);
            emit(Event::StepOn(Contact {
                block: "minecraft:hay_block",
                ..c
            }));
        });
        assert_eq!(fall(c, 12.0, 10.0), 10.0);
        assert_eq!(SEEN.get(), 3);
        in_level(Some("minecraft:the_nether"), || {
            assert_eq!(fall(c, 12.0, 99.0), 99.0);
            assert_eq!(fall(c, 12.0, 98.0), 0.0);
        });
        let _ =
            std::panic::catch_unwind(|| in_level(Some("minecraft:the_nether"), || panic!("test")));
        assert_eq!(fall(c, 12.0, 10.0), 10.0);
    }

    /// `checkInsideBlocks`' `isPrecise`: a long move, or the box at the target
    /// touching the block.
    #[test]
    fn precision_follows_the_move_length_and_the_box() {
        use horizon_core::geometry::{Aabb, Vec3};
        let at = Aabb::new(Vec3::new(2.25, 2.0, 3.25), Vec3::new(2.75, 2.5, 3.75));
        let block = BlockPos::new(2, 2, 3);
        let from = Vec3::new(2.5, 2.0, 3.5);
        assert!(is_precise(from, from, at, block));
        assert!(!is_precise(from, from, at, BlockPos::new(1, 2, 3)));
        assert!(is_precise(
            Vec3::new(0.5, 2.0, 3.5),
            from,
            at,
            BlockPos::new(1, 2, 3)
        ));
    }
}
