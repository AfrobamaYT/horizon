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
    pub state: BlockStateId,
    pub position: BlockPos,
}

/// What a landing hook may read of the level it happens in: the blocks, the registry
/// that names them and the level's height range. The simulation (the player's motion, a
/// mob's tick) holds the world it is moving through, so this is a borrow of that.
pub trait ContactLevel {
    fn block(&self, position: BlockPos) -> BlockStateId;
    fn registry(&self) -> &horizon_world::registry::BlockRegistry;
    fn min_y(&self) -> i32;
    fn height(&self) -> i32;
}

/// Where the contacts are answered: the game binds these to the hooks of the block
/// parts, and says which block states have a part for each.
pub struct Hooks {
    /// `Block.stepOn` for a mod block, in the dimension.
    pub step_on: fn(&str, Contact<'_>),
    pub any_inside: fn() -> bool,
    pub inside_listens: fn(BlockStateId) -> bool,
    pub fall_listens: fn(BlockStateId) -> bool,
    pub bounce_listens: fn(BlockStateId) -> bool,
    /// `Block.fallOn`: change `damage`, the fall damage `causeFallDamage` calculated.
    pub fall_on: fn(&str, &dyn ContactLevel, Contact<'_>, f64, &mut f32),
    /// `Block.bounceOn`: change `movement`, the movement the bounce leaves the entity.
    pub bounce_on: fn(&str, &dyn ContactLevel, Contact<'_>, f64, &mut horizon_core::geometry::Vec3),
}

pub static HOOKS: OnceLock<Hooks> = OnceLock::new();

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

/// `Block.stepOn` for a mod block: the entity stands on `contact`'s block.
pub fn step_on(block: &str, contact: Contact<'_>) {
    if block.starts_with("minecraft:") {
        return;
    }
    if let Some(dimension) = LEVEL.get()
        && let Some(hooks) = HOOKS.get()
    {
        (hooks.step_on)(dimension, contact);
    }
}

/// Whether a part of the block `state` answers `fall_on`.
pub fn fall_listens(state: BlockStateId) -> bool {
    HOOKS.get().is_some_and(|hooks| (hooks.fall_listens)(state))
}

/// `Block.fallOn`: the fall damage `damage` calculated for the entity that landed on
/// `contact`'s block, after any part of that block changed it. A non-finite answer is
/// ignored and a negative one is zero. Nothing runs off the server or without a part.
pub fn fall(level: &dyn ContactLevel, contact: Contact<'_>, distance: f64, damage: f32) -> f32 {
    let (Some(dimension), Some(hooks)) = (LEVEL.get(), HOOKS.get()) else {
        return damage;
    };
    if !(hooks.fall_listens)(contact.state) {
        return damage;
    }
    let mut answer = damage;
    (hooks.fall_on)(dimension, level, contact, distance, &mut answer);
    if answer.is_finite() {
        answer.max(0.0)
    } else {
        damage
    }
}

/// `Block.bounceOn`: the movement a bounce off `contact`'s block leaves, after any part of
/// that block changed it. A non-finite answer is ignored.
pub fn bounce(
    level: &dyn ContactLevel,
    contact: Contact<'_>,
    fall_distance: f64,
    movement: horizon_core::geometry::Vec3,
) -> horizon_core::geometry::Vec3 {
    let (Some(dimension), Some(hooks)) = (LEVEL.get(), HOOKS.get()) else {
        return movement;
    };
    if !(hooks.bounce_listens)(contact.state) {
        return movement;
    }
    let mut answer = movement;
    (hooks.bounce_on)(dimension, level, contact, fall_distance, &mut answer);
    if answer.x.is_finite() && answer.y.is_finite() && answer.z.is_finite() {
        answer
    } else {
        movement
    }
}

/// Note an entity inside a block, if a part there listens; nothing is kept
/// otherwise, so a world without such a part pays one test per block.
pub fn inside(entity: i32, state: BlockStateId, position: BlockPos, precise: bool) {
    if LEVEL.get().is_some()
        && HOOKS
            .get()
            .is_some_and(|hooks| (hooks.inside_listens)(state))
    {
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
    let far = 0.999_990_000_000_252_6_f64;
    from.distance_squared(to) > far * far
        || at_target.intersects(horizon_core::geometry::Aabb::block(position))
}

/// Reuse the entity block sweep for item and projectile ticks without an effect collector.
pub fn swept_inside(
    entity: i32,
    from: horizon_core::geometry::Vec3,
    to: horizon_core::geometry::Vec3,
    bounds: horizon_core::geometry::Aabb,
    mut block: impl FnMut(BlockPos) -> BlockStateId,
) {
    if LEVEL.get().is_none() || !HOOKS.get().is_some_and(|hooks| (hooks.any_inside)()) {
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

    struct Flat;
    impl ContactLevel for Flat {
        fn block(&self, _: BlockPos) -> BlockStateId {
            BlockStateId(0)
        }
        fn registry(&self) -> &horizon_world::registry::BlockRegistry {
            unreachable!()
        }
        fn min_y(&self) -> i32 {
            0
        }
        fn height(&self) -> i32 {
            16
        }
    }

    fn contact() -> Contact<'static> {
        Contact {
            entity: 1,
            kind: "minecraft:player",
            state: BlockStateId(0),
            position: BlockPos::new(0, 0, 0),
        }
    }

    /// A part's fall answer is clamped at zero, and a non-finite one is ignored; a
    /// bounce answer is used unless it is non-finite. Nothing runs outside a level.
    #[test]
    fn landing_answers_are_checked() {
        use horizon_core::geometry::Vec3;
        static HOOKS_FOR_TEST: Hooks = Hooks {
            step_on: |_, _| {},
            any_inside: || false,
            inside_listens: |_| false,
            fall_listens: |_| true,
            bounce_listens: |_| true,
            fall_on: |_, _, _, distance, damage| *damage = (distance - 10.0) as f32,
            bounce_on: |_, _, _, _, movement| movement.y *= 0.5,
        };
        let _ = HOOKS.set(Hooks { ..HOOKS_FOR_TEST });
        assert_eq!(fall(&Flat, contact(), 4.0, 3.0), 3.0);
        in_level(Some("minecraft:overworld"), || {
            assert_eq!(fall(&Flat, contact(), 4.0, 3.0), 0.0);
            assert_eq!(fall(&Flat, contact(), 15.0, 3.0), 5.0);
            assert_eq!(
                bounce(&Flat, contact(), 4.0, Vec3::new(0.0, 0.8, 0.0)),
                Vec3::new(0.0, 0.4, 0.0)
            );
        });
    }
}
