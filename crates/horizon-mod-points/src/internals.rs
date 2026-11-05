//! The game's internals as a wasm mod reaches them (#589 W): the table the staged game generates
//! (`horizon_client::__internals::TABLE`) and what crosses through it. Only the game's generated
//! code and the wasm runtime use it; a mod sees its own generated view instead.
//!
//! Every pointer here is derived, within one call into a mod, from that call's roots (the slot's
//! own borrows); none is kept past the call. The runtime keeps that rule; this module only names
//! the shapes.

/// A copy of a plain game value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `()`, or a unit case.
    None,
    B(bool),
    C(char),
    S(i64),
    U(u64),
    F(f64),
    Str(String),
    /// An array, a slice, a list or a set's elements; a map's entries, each a two-field `Rec`.
    Seq(Vec<Value>),
    /// A tuple's or a struct's fields, in order.
    Rec(Vec<Value>),
    /// An enum's case, by index, with its fields; an `Option` is case 0 for `None` and 1 for `Some`.
    Var(u32, Vec<Value>),
}

/// Why a step, read, write or call can't be done.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// The node's call is over, or its path leads nowhere now.
    Gone(String),
    /// A value of the wrong shape for its place.
    Changed(String),
    /// Not on this node: another enum case, an index past the end, a key not present, a write
    /// under a shared node, or an operation its type doesn't have.
    NotHere(String),
}

/// One operation on a node. A field is numbered in its type's declaration order; an enum's fields
/// are numbered across its cases, in order.
#[derive(Debug)]
pub enum Op<'v> {
    /// A field whose type is walked: answers its node.
    Field(u32),
    /// A field whose type is a copy: answers its value.
    Get(u32),
    /// Writes a copy-typed field.
    Set(u32, Value),
    /// A list's element: answers its node.
    Index(u64),
    /// A map's entry by key: answers its node.
    Key(&'v Value),
    /// The number of elements or entries.
    Len,
    /// An enum's or an `Option`'s case.
    Case,
    /// The whole node, when its type has a copy.
    Read,
}

/// What an operation answers.
#[derive(Debug, PartialEq)]
pub enum Reply {
    /// A node, of a type in the table, and whether it may be written (`false` under an `Arc` or
    /// `Rc`, or under a `&` the game holds).
    Node(*mut (), u32, bool),
    Value(Value),
    Done,
}

/// A type's operations, on a pointer to a value of that type, which may be written (and reached
/// through `&mut`) only when the flag says so.
pub type NodeFn = unsafe fn(*mut (), bool, Op<'_>) -> Result<Reply, Refusal>;

/// A function's argument: a copy, or a node standing for a reference argument.
#[derive(Debug)]
pub enum Arg {
    Value(Value),
    Node(*mut ()),
}

/// A declared function's call: its receiver's node (none for a free function) and its arguments.
/// A reference it returns comes back as a node, valid until the next call.
pub type ThunkFn = unsafe fn(Option<*mut ()>, Vec<Arg>) -> Result<Reply, Refusal>;

/// A declared function.
pub struct Thunk {
    /// Its `internals.json` path.
    pub path: &'static str,
    /// Which nodes it writes through: bit 0 the receiver, bit `i + 1` argument `i`. The runtime
    /// refuses a shared node there.
    pub writes: u64,
    pub call: ThunkFn,
}

/// The staged game's internals.
pub struct Table {
    /// Each walked type's operations, by its path as `internals.json` spells it, or for a type
    /// that isn't listed (a list, a map, an option), as the game's crate root spells it.
    pub types: &'static [(&'static str, NodeFn)],
    pub thunks: &'static [Thunk],
}

/// The wrong shape for `what`.
pub fn changed(what: &str) -> Refusal {
    Refusal::Changed(what.to_owned())
}

/// Not on this node.
pub fn not_here(what: &str) -> Refusal {
    Refusal::NotHere(what.to_owned())
}

/// A whole number from `value`, if it fits `T`.
pub fn int<T: TryFrom<i64> + TryFrom<u64>>(value: Value) -> Result<T, Refusal> {
    match value {
        Value::S(n) => T::try_from(n).map_err(|_| changed("an integer out of range")),
        Value::U(n) => T::try_from(n).map_err(|_| changed("an integer out of range")),
        _ => Err(changed("an integer")),
    }
}

/// Exactly `N` fields of a `Rec` or a case, or arguments.
pub fn fields<T, const N: usize>(values: Vec<T>) -> Result<[T; N], Refusal> {
    values
        .try_into()
        .map_err(|_| changed("a count of fields or arguments"))
}

/// A `Rec`'s fields.
pub fn rec(value: Value) -> Result<Vec<Value>, Refusal> {
    match value {
        Value::Rec(fields) => Ok(fields),
        _ => Err(changed("a record")),
    }
}

/// A `Seq`'s elements.
pub fn seq(value: Value) -> Result<Vec<Value>, Refusal> {
    match value {
        Value::Seq(items) => Ok(items),
        _ => Err(changed("a sequence")),
    }
}

/// An argument given as a copy.
pub fn value(arg: Arg) -> Result<Value, Refusal> {
    match arg {
        Arg::Value(value) => Ok(value),
        Arg::Node(_) => Err(changed("a value, not a node")),
    }
}

/// An argument given as a node.
pub fn node(arg: Arg) -> Result<*mut (), Refusal> {
    match arg {
        Arg::Node(node) => Ok(node),
        Arg::Value(_) => Err(changed("a node, not a value")),
    }
}
