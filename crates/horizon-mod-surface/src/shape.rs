//! The one shape function: a type's wire layout hash, from a canonical description of the type.
//!
//! # The description
//!
//! A [`Definition`] describes one named type: its number of type parameters and its [`Body`]. A
//! [`Shape`] describes the type of one field. Definitions live in a table keyed by any string the
//! producer chooses (the generator uses the type's path); **keys and type names never enter the
//! hash**, so renaming a type changes no shape. Field names and variant names do, as does their
//! order.
//!
//! # The canonical text
//!
//! `shape(key)` is the first 128 bits, as 32 lowercase hex digits, of the SHA-256 of the
//! canonical text of the type at `key` with its parameters as `param(<i>)`. The text is ASCII,
//! with no spaces; `<n>:<s>` writes a name `s` with its length in bytes `n` in decimal.
//!
//! | Description | Text |
//! |---|---|
//! | [`Scalar`] | its name (`u32`, `str`, `duration`, `unit`, …) |
//! | `List(t)`, `Set(t)`, `Option(t)` | `list(t)`, `set(t)`, `option(t)` |
//! | `Map(k, v)`, `Result(a, b)` | `map(k,v)`, `result(a,b)` |
//! | `Tuple(a, b, …)` | `tuple(a,b,…)` |
//! | `Array(t, n)` | `array(t;n)` |
//! | `Param(i)` | the text of argument `i` of the type being described |
//! | `Opaque(s)` | `opaque(<n>:<s>)`: a type the description does not look into |
//! | `Named(key, args)` | `rec(<d>)` when the same key with the same argument texts is being described `d` levels up (1 = the type directly around it); otherwise `type(<shape of key with args>)` |
//! | `Body::Record(fields)` | `record{<n>:<name>:<t>,…}` |
//! | `Body::Tuple(fields)` | `tuple{<t>,…}` |
//! | `Body::Unit` | `unit{}` |
//! | `Body::Enum(variants)` | `enum{<n>:<name><body>,…}`, each variant's body as above |
//! | `Body::Via(t)` | `via(t)`: a type that crosses as another |
//!
//! A shape is computed once per key and argument list, except for a type in a cycle, which is
//! described afresh each time it is reached: the result never depends on the order shapes are
//! asked for.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A type with a fixed layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scalar {
    Unit,
    Bool,
    Char,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    F32,
    F64,
    /// Any string form: `String`, `&str`, `Box<str>`, `Arc<str>`, `Cow<str>`.
    Str,
    /// `std::time::Duration`.
    Duration,
}

impl Scalar {
    /// Its name in the canonical text.
    pub fn name(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::Char => "char",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
            Self::Usize => "usize",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I128 => "i128",
            Self::Isize => "isize",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::Str => "str",
            Self::Duration => "duration",
        }
    }
}

/// The type of one field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shape {
    Scalar(Scalar),
    /// `Vec<T>`, `VecDeque<T>`, `[T]`, `Box<[T]>`, `Arc<[T]>`.
    List(Box<Shape>),
    Set(Box<Shape>),
    Map(Box<Shape>, Box<Shape>),
    Option(Box<Shape>),
    Result(Box<Shape>, Box<Shape>),
    Tuple(Vec<Shape>),
    Array(Box<Shape>, u64),
    /// A described type, by key, with its type arguments.
    Named(String, Vec<Shape>),
    /// Type parameter `i` of the type being described.
    Param(usize),
    /// A type the description does not look into, as written.
    Opaque(String),
}

/// A named type's layout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Body {
    Record(Vec<(String, Shape)>),
    Tuple(Vec<Shape>),
    Unit,
    Enum(Vec<(String, Body)>),
    /// It crosses as another type (`#[wire(via = T)]`).
    Via(Shape),
}

/// One named type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Definition {
    /// How many type parameters it has.
    pub params: usize,
    pub body: Body,
}

/// Why a shape could not be computed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShapeError {
    /// A `Named` key with no definition.
    Undefined(String),
    /// A `Named` with a different number of arguments than its definition has parameters.
    Arguments {
        key: String,
        expected: usize,
        found: usize,
    },
    /// A `Param` index outside the type's parameters.
    Param { key: String, index: usize },
}

impl std::fmt::Display for ShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Undefined(key) => write!(f, "no definition for type {key}"),
            Self::Arguments {
                key,
                expected,
                found,
            } => write!(f, "{key} takes {expected} type arguments, given {found}"),
            Self::Param { key, index } => write!(f, "{key} has no type parameter {index}"),
        }
    }
}

impl std::error::Error for ShapeError {}

/// Computes shapes over one table of definitions, remembering each finished one.
pub struct Shapes<'a> {
    table: &'a BTreeMap<String, Definition>,
    done: BTreeMap<(String, Vec<String>), String>,
}

/// One type being described: its key, its argument texts, and the lowest stack level a `rec`
/// inside it referred to.
struct Frame {
    key: String,
    args: Vec<String>,
    reaches: usize,
}

impl<'a> Shapes<'a> {
    pub fn new(table: &'a BTreeMap<String, Definition>) -> Self {
        Self {
            table,
            done: BTreeMap::new(),
        }
    }

    /// The shape of the type at `key`, its parameters left as parameters.
    pub fn shape(&mut self, key: &str) -> Result<String, ShapeError> {
        let definition = self
            .table
            .get(key)
            .ok_or_else(|| ShapeError::Undefined(key.to_owned()))?;
        let args = (0..definition.params)
            .map(|i| format!("param({i})"))
            .collect();
        let text = self.named(key, args, &mut Vec::new())?;
        // With nothing around it, the type is never a `rec`.
        Ok(text["type(".len()..text.len() - 1].to_owned())
    }

    fn named(
        &mut self,
        key: &str,
        args: Vec<String>,
        stack: &mut Vec<Frame>,
    ) -> Result<String, ShapeError> {
        if let Some(level) = stack
            .iter()
            .rposition(|frame| frame.key == key && frame.args == args)
        {
            let depth = stack.len() - level;
            for frame in &mut stack[level + 1..] {
                frame.reaches = frame.reaches.min(level);
            }
            if let Some(top) = stack.last_mut() {
                top.reaches = top.reaches.min(level);
            }
            return Ok(format!("rec({depth})"));
        }
        let memo = (key.to_owned(), args);
        if let Some(done) = self.done.get(&memo) {
            return Ok(format!("type({done})"));
        }
        let definition = self
            .table
            .get(key)
            .ok_or_else(|| ShapeError::Undefined(key.to_owned()))?;
        if definition.params != memo.1.len() {
            return Err(ShapeError::Arguments {
                key: key.to_owned(),
                expected: definition.params,
                found: memo.1.len(),
            });
        }
        let level = stack.len();
        stack.push(Frame {
            key: memo.0.clone(),
            args: memo.1.clone(),
            reaches: usize::MAX,
        });
        let mut text = String::new();
        let written = self.body(&definition.body, &mut text, stack);
        let frame = stack.pop().expect("pushed above");
        written?;
        let hash = digest(&text);
        // Its text refers to no type at its own level or around it, so it is in no cycle and its
        // text is the same in every context. A type in a cycle is described afresh each time:
        // remembering it would make other shapes depend on the order they were asked for.
        if frame.reaches > level {
            self.done.insert(memo, hash.clone());
        }
        if let Some(parent) = stack.last_mut() {
            parent.reaches = parent.reaches.min(frame.reaches);
        }
        Ok(format!("type({hash})"))
    }

    fn body(
        &mut self,
        body: &Body,
        out: &mut String,
        stack: &mut Vec<Frame>,
    ) -> Result<(), ShapeError> {
        match body {
            Body::Record(fields) => {
                out.push_str("record{");
                for (i, (name, shape)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    let _ = write!(out, "{}:{name}:", name.len());
                    self.field(shape, out, stack)?;
                }
                out.push('}');
            }
            Body::Tuple(fields) => {
                out.push_str("tuple{");
                for (i, shape) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    self.field(shape, out, stack)?;
                }
                out.push('}');
            }
            Body::Unit => out.push_str("unit{}"),
            Body::Enum(variants) => {
                out.push_str("enum{");
                for (i, (name, body)) in variants.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    let _ = write!(out, "{}:{name}", name.len());
                    self.body(body, out, stack)?;
                }
                out.push('}');
            }
            Body::Via(shape) => {
                out.push_str("via(");
                self.field(shape, out, stack)?;
                out.push(')');
            }
        }
        Ok(())
    }

    fn field(
        &mut self,
        shape: &Shape,
        out: &mut String,
        stack: &mut Vec<Frame>,
    ) -> Result<(), ShapeError> {
        let wrap = |this: &mut Self,
                    name: &str,
                    parts: &[&Shape],
                    out: &mut String,
                    stack: &mut Vec<Frame>|
         -> Result<(), ShapeError> {
            out.push_str(name);
            out.push('(');
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                this.field(part, out, stack)?;
            }
            out.push(')');
            Ok(())
        };
        match shape {
            Shape::Scalar(scalar) => out.push_str(scalar.name()),
            Shape::List(t) => wrap(self, "list", &[t], out, stack)?,
            Shape::Set(t) => wrap(self, "set", &[t], out, stack)?,
            Shape::Option(t) => wrap(self, "option", &[t], out, stack)?,
            Shape::Map(k, v) => wrap(self, "map", &[k, v], out, stack)?,
            Shape::Result(a, b) => wrap(self, "result", &[a, b], out, stack)?,
            Shape::Tuple(parts) => {
                let parts: Vec<&Shape> = parts.iter().collect();
                wrap(self, "tuple", &parts, out, stack)?;
            }
            Shape::Array(t, n) => {
                out.push_str("array(");
                self.field(t, out, stack)?;
                let _ = write!(out, ";{n})");
            }
            Shape::Param(index) => {
                let frame = stack.last().expect("a field is inside a type");
                let arg = frame.args.get(*index).ok_or_else(|| ShapeError::Param {
                    key: frame.key.clone(),
                    index: *index,
                })?;
                out.push_str(arg);
            }
            Shape::Opaque(written) => {
                let _ = write!(out, "opaque({}:{written})", written.len());
            }
            Shape::Named(key, args) => {
                let mut texts = Vec::with_capacity(args.len());
                for arg in args {
                    let mut text = String::new();
                    self.field(arg, &mut text, stack)?;
                    texts.push(text);
                }
                let text = self.named(key, texts, stack)?;
                out.push_str(&text);
            }
        }
        Ok(())
    }
}

/// The first 128 bits of SHA-256 of `text`, as 32 lowercase hex digits.
pub(crate) fn digest(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(text.as_bytes());
    let mut out = String::with_capacity(32);
    for byte in &hash[..16] {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
#[path = "shape_tests.rs"]
mod tests;
