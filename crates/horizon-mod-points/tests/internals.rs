#![cfg(feature = "wasm-host")]
use mod_points::internals::{Cell, Refusal, Value, flatten, unflatten};

#[test]
fn nested_values_cross_in_preorder() {
    let value = Value::Rec(vec![
        Value::Seq(vec![Value::None, Value::B(true), Value::C('λ')]),
        Value::Var(2, vec![Value::S(-9), Value::U(u64::MAX), Value::F(0.5)]),
        Value::Str("a field".into()),
        Value::Rec(vec![]),
    ]);
    let cells = flatten(value.clone());
    assert!(matches!(cells[0], Cell::Rec(4)));
    assert!(matches!(cells[1], Cell::Seq(3)));
    assert!(matches!(cells[5], Cell::Var((2, 3))));
    assert_eq!(unflatten(cells).unwrap(), value);
}

#[test]
fn malformed_cells_are_refused_not_panicked() {
    for cells in [
        vec![],
        vec![Cell::Seq(u32::MAX)],
        vec![Cell::Rec(1), Cell::Seq(1)],
        vec![Cell::Var((1, 2)), Cell::None],
        vec![Cell::None, Cell::None],
        vec![Cell::Seq(0), Cell::None],
    ] {
        assert!(matches!(unflatten(cells), Err(Refusal::Changed(_))));
    }
    let mut deep: Vec<Cell> = (0..256).map(|_| Cell::Seq(1)).collect();
    deep.push(Cell::None);
    assert!(matches!(unflatten(deep), Err(Refusal::Changed(_))));
}
