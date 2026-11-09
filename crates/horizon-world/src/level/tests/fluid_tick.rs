//! Water left alone by a mod that took over its tick, and water that spreads as vanilla.
use super::*;

#[test]
fn water_spreads_on_a_floor() {
    let mut w = world!();
    w.floor(0, 10, "stone");
    let water = w.id("water", &[]);
    w.set(BlockPos::new(0, 1, 0), water);
    for _ in 0..60 {
        w.tick();
    }
    for d in 1..=7 {
        let pos = BlockPos::new(d, 1, 0);
        assert_eq!(
            w.prop(pos, "level"),
            d.to_string(),
            "flow at distance {d}: {}",
            w.name(pos)
        );
    }
    assert_eq!(w.get(BlockPos::new(8, 1, 0)), w.b().air());
    // Diagonals follow Manhattan distance.
    assert_eq!(w.prop(BlockPos::new(2, 1, 2), "level"), "4");
}

#[test]
fn refused_fluid_tick_leaves_the_water_alone() {
    let mut w = world!();
    w.floor(0, 10, "stone");
    let source = BlockPos::new(0, 1, 0);
    w.level.guarded.insert(source);
    let water = w.id("water", &[]);
    w.set(source, water);
    for _ in 0..60 {
        w.tick();
    }
    assert_eq!(w.get(source), water);
    assert_eq!(w.get(BlockPos::new(1, 1, 0)), w.b().air());
    // Allowed again, its tick is gone: nothing was left scheduled to run it.
    w.level.guarded.clear();
    for _ in 0..60 {
        w.tick();
    }
    assert_eq!(w.get(BlockPos::new(1, 1, 0)), w.b().air());
}
