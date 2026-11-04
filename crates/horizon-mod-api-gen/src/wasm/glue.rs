//! The Rust on both sides of the WIT world: conversions between `mod_api`'s types and the
//! bindings' records, the guest's host (`Guest`, every imported trait forwarded to its import)
//! and exports (each export calling the mod), and the game's imports (each forwarding to the
//! host of the call's frame) and mod proxies (each trait method calling its export).
//!
//! Lowering takes `&R` and builds the wire value; lifting takes the wire value and builds `R`,
//! fallibly (a value the other side sent may not be one `R` can hold). A type that borrows is
//! lifted where it is used: each borrowed part into a local first, then the value pointing at
//! them.
use super::ty::{
    Case, Def, Kind, List, Payload, Str, Ty, Types, bound_ident, bound_type, wit_ident,
};
use super::wit::{self, Signature, writable};
use heck::ToSnakeCase;
use std::fmt::Write;

/// The last arm of a generated match over a game enum's cases, all read from its source when
/// the glue is generated; it only answers `#[non_exhaustive]` on another crate's enum.
const EVERY_CASE: &str = r#"_ => unreachable!("every case was read from the enum's source")"#;

/// The wire module of the shared types, as both bindings generators lay it out.
const W: &str = "horizon::mod_api::types";
pub(super) const WIRE: &str = W;

/// `def`'s Rust path, as code inside `mod_api` names it.
pub(super) fn rust_path(def: &Def) -> String {
    let path = match def.rust.strip_prefix("mod_api::") {
        Some(rest) => format!("crate::{rest}"),
        None => def.rust.clone(),
    };
    match def.lifetimes {
        0 => path,
        n => format!("{path}<{}>", vec!["'_"; n].join(", ")),
    }
}

fn wire_path(def: &Def) -> String {
    format!("{W}::{}", bound_type(&def.wit))
}

/// Whether a type holds a lifetime: it can't be lifted on its own.
fn borrows(types: &Types<'_>, ty: &Ty) -> bool {
    match ty {
        Ty::Str(Str::Borrowed) | Ty::List(_, List::Slice) | Ty::Ref(..) | Ty::Dyn(..) => true,
        Ty::List(item, _) | Ty::Array(item, _) | Ty::Option(item) | Ty::Boxed(item) => {
            borrows(types, item)
        }
        Ty::Result(a, b) => borrows(types, a) || borrows(types, b),
        Ty::Tuple(items) => items.iter().any(|item| borrows(types, item)),
        Ty::Named(def) => types.defs[*def].lifetimes > 0,
        _ => false,
    }
}

/// Whether a type holds a record with a lifetime, which has nothing to borrow from when the
/// game hands it back to the mod.
pub(super) fn borrowed_record(types: &Types<'_>, ty: &Ty) -> bool {
    match ty {
        Ty::List(item, _) | Ty::Array(item, _) | Ty::Option(item) | Ty::Boxed(item) => {
            borrowed_record(types, item)
        }
        Ty::Result(a, b) => borrowed_record(types, a) || borrowed_record(types, b),
        Ty::Tuple(items) => items.iter().any(|item| borrowed_record(types, item)),
        Ty::Named(def) => types.defs[*def].lifetimes > 0,
        _ => false,
    }
}

/// An expression lowering the value `e` names, owned: a returned handler moves into the mod's
/// table; anything else is lowered by reference.
pub(super) fn lower_owned(types: &Types<'_>, ty: &Ty, e: &str) -> String {
    match ty {
        Ty::Handler(name, true) => keep_handler(types, name, &format!("std::sync::Arc::from({e})")),
        Ty::Option(item) if matches!(**item, Ty::Handler(..)) => {
            format!("{e}.map(|x| {})", lower_owned(types, item, "x"))
        }
        ty => lower(types, ty, &format!("(&{e})")),
    }
}

/// The guest keeping handler `arc` (an `Arc<dyn name>`) under each trait it answers as, and the
/// record the game gets: its index and the hooks it listens to as any of them.
fn keep_handler(types: &Types<'_>, name: &str, arc: &str) -> String {
    let views = types
        .views
        .iter()
        .find(|(owner, _)| owner == name)
        .map_or(&[][..], |(_, list)| list.as_slice());
    let kept: Vec<String> = views
        .iter()
        .map(|view| {
            format!(
                "super::view::<dyn {}>(h.clone())",
                super::sides::trait_path(view)
            )
        })
        .collect();
    let lists: Vec<String> = views
        .iter()
        .filter_map(|view| super::SUBSCRIBED.iter().find(|(owner, _)| owner == view))
        .map(|(_, list)| format!("h.{list}().iter()"))
        .collect();
    let subscribed = match lists.split_first() {
        None => "Vec::new()".to_string(),
        Some((first, rest)) => format!(
            "{first}{}.flatten().map(|h| hook_name(*h).to_string()).collect()",
            rest.iter()
                .map(|l| format!(".chain({l})"))
                .collect::<String>()
        ),
    };
    format!(
        "{{ let h = {arc}; {W}::Handler {{ subscribed: {subscribed}, index: super::add_handler(vec![{}]) }} }}",
        kept.join(", ")
    )
}

/// An expression lowering `e`, of type `&R`, to its wire value.
pub(super) fn lower(types: &Types<'_>, ty: &Ty, e: &str) -> String {
    match ty {
        Ty::Unit | Ty::Dyn(..) => "()".into(),
        Ty::Prim(rust, wire)
            if rust == wire
                || (*rust == "i32" || *rust == "i64" || *rust == "i8" || *rust == "i16") =>
        {
            format!("*{e}")
        }
        Ty::Prim(_, wire) => format!("(*{e} as {})", wire_prim(wire)),
        Ty::Wide(_) => format!("{{ let v = *{e} as u128; ((v >> 64) as u64, v as u64) }}"),
        Ty::Duration => format!("u64::try_from({e}.as_nanos()).unwrap_or(u64::MAX)"),
        // Only a mod hands one over: the guest keeps it under its views and sends its index.
        Ty::Handler(name, false) => keep_handler(types, name, &format!("{e}.clone()")),
        Ty::Handler(_, true) => unreachable!("a boxed handler is lowered owned (`lower_owned`)"),
        Ty::Str(_) => format!("{e}.to_string()"),
        Ty::List(item, _) | Ty::Array(item, _) => {
            format!(
                "{e}.iter().map(|x| {}).collect::<Vec<_>>()",
                lower(types, item, "x")
            )
        }
        Ty::Option(item) => format!("{e}.as_ref().map(|x| {})", lower(types, item, "x")),
        Ty::Result(ok, err) => format!(
            "match {e} {{ Ok(x) => Ok({}), Err(x) => Err({}) }}",
            lower_or_unit(types, ok, "x"),
            lower_or_unit(types, err, "x")
        ),
        Ty::Tuple(items) => {
            let names: Vec<String> = (0..items.len()).map(|i| format!("t{i}")).collect();
            let parts: Vec<String> = items
                .iter()
                .zip(&names)
                .filter(|(item, _)| crosses(types, item))
                .map(|(item, name)| lower(types, item, name))
                .collect();
            format!(
                "{{ let ({}) = {e}; ({},) }}",
                names.join(", "),
                parts.join(", ")
            )
        }
        Ty::Ref(inner, _) => lower(types, inner, &format!("(&**{e})")),
        Ty::Boxed(inner) => lower(types, inner, &format!("(&**{e})")),
        Ty::Named(def) => format!("lower_{def}({e})"),
    }
}

fn lower_or_unit(types: &Types<'_>, ty: &Ty, e: &str) -> String {
    if crosses(types, ty) {
        lower(types, ty, e)
    } else {
        "()".into()
    }
}

fn wire_prim(wire: &str) -> &'static str {
    match wire {
        "u64" => "u64",
        "s64" => "i64",
        _ => unreachable!("only usize and isize widen"),
    }
}

/// Whether any value crosses for `ty`.
fn crosses(types: &Types<'_>, ty: &Ty) -> bool {
    wit::wit(types, ty).ok().flatten().is_some()
}

/// An expression lifting the wire value `e` to an owned `ty`, inside a function returning
/// `Result<_, Refused>`.
pub(super) fn lift(types: &Types<'_>, ty: &Ty, e: &str) -> String {
    match ty {
        Ty::Unit => "()".into(),
        Ty::Prim(rust, wire)
            if wire.starts_with(['u', 's']) && (*rust == "usize" || *rust == "isize") =>
        {
            format!("({e} as {rust})")
        }
        Ty::Prim(..) => e.into(),
        Ty::Wide(rust) => {
            format!("{{ let (h, l) = {e}; ((h as u128) << 64 | l as u128) as {rust} }}")
        }
        Ty::Duration => format!("std::time::Duration::from_nanos({e})"),
        // Only the game takes one: its proxy, calling back into the mod that handed it over.
        Ty::Handler(name, boxed) => {
            let path = super::sides::trait_path(name);
            if *boxed {
                format!("(Box::new(super::handler_of({e})?) as Box<dyn {path}>)")
            } else {
                format!(
                    "(std::sync::Arc::new(super::handler_of({e})?) as std::sync::Arc<dyn {path}>)"
                )
            }
        }
        Ty::Str(Str::Owned) => e.into(),
        Ty::Str(Str::Static) => format!("super::intern({e})"),
        Ty::Str(_) => format!("{e}.into()"),
        Ty::List(item, kind) => {
            let vec = format!(
                "{e}.into_iter().map(|x| Ok({})).collect::<Result<Vec<_>, Refused>>()?",
                lift(types, item, "x")
            );
            match kind {
                List::Vec => vec,
                List::Map(map) => format!("{vec}.into_iter().collect::<{map}<_, _>>()"),
                _ => format!("{vec}.into()"),
            }
        }
        Ty::Array(item, _) => format!(
            "super::array({e}.into_iter().map(|x| Ok({})).collect::<Result<Vec<_>, Refused>>()?)?",
            lift(types, item, "x")
        ),
        Ty::Option(item) => format!(
            "match {e} {{ Some(x) => Some({}), None => None }}",
            lift(types, item, "x")
        ),
        Ty::Result(ok, err) => format!(
            "match {e} {{ Ok(x) => Ok({}), Err(x) => Err({}) }}",
            lift_or_unit(types, ok, "x"),
            lift_or_unit(types, err, "x")
        ),
        Ty::Tuple(items) => {
            let mut names = Vec::new();
            let mut parts = Vec::new();
            for (i, item) in items.iter().enumerate() {
                if crosses(types, item) {
                    names.push(format!("t{i}"));
                    parts.push(lift(types, item, &format!("t{i}")));
                } else {
                    parts.push("()".into());
                }
            }
            format!(
                "{{ let ({},) = {e}; ({},) }}",
                names.join(", "),
                parts.join(", ")
            )
        }
        Ty::Boxed(inner) => format!("Box::new({})", lift(types, inner, e)),
        Ty::Named(def) => format!("lift_{def}({e})?"),
        Ty::Ref(..) | Ty::Dyn(..) => unreachable!("borrowed values are lifted in place"),
    }
}

fn lift_or_unit(types: &Types<'_>, ty: &Ty, e: &str) -> String {
    if crosses(types, ty) {
        lift(types, ty, e)
    } else {
        "()".into()
    }
}

/// Lifts wire values where they are used. Borrowed parts go into the call's arena `keep`
/// (`super::Keep`, declared first in every generated call), so a lifted value is one
/// expression. `object` gives the expression standing in for each `dyn` trait object: the
/// frame's on the game's side, the guest's own forwarding value on the mod's.
pub(super) struct InPlace<'t, 'a> {
    pub(super) types: &'t Types<'a>,
    pub(super) object: &'t dyn Fn(&str, bool) -> String,
}

impl<'t, 'a> InPlace<'t, 'a> {
    pub(super) fn new(types: &'t Types<'a>, object: &'t dyn Fn(&str, bool) -> String) -> Self {
        Self { types, object }
    }

    /// The value for `ty` from `e`: its wire value, or `()` where none crosses.
    pub(super) fn value(&self, ty: &Ty, e: &str) -> Result<String, String> {
        let types = self.types;
        if !borrows(types, ty) {
            return Ok(lift(types, ty, e));
        }
        let collect = |item: &Ty| -> Result<String, String> {
            Ok(format!(
                "{e}.into_iter().map(|x| Ok({})).collect::<Result<Vec<_>, Refused>>()?",
                self.value(item, "x")?
            ))
        };
        Ok(match ty {
            Ty::Dyn(name, mutable) => (self.object)(name, *mutable),
            Ty::Str(Str::Borrowed) => format!("keep.str({e})"),
            Ty::List(item, List::Slice) => format!("keep.slice({})", collect(item)?),
            Ty::List(item, _) => format!("{}.into()", collect(item)?),
            Ty::Array(item, _) => format!("super::array({})?", collect(item)?),
            Ty::Option(item) => format!(
                "match {e} {{ Some(x) => Some({}), None => None }}",
                self.value(item, "x")?
            ),
            Ty::Result(ok, err) => format!(
                "match {e} {{ Ok(x) => Ok({}), Err(x) => Err({}) }}",
                self.value(ok, "x")?,
                self.value(err, "x")?
            ),
            Ty::Tuple(items) => {
                let mut names = Vec::new();
                let mut parts = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    if crosses(types, item) {
                        names.push(format!("t{i}"));
                        parts.push(self.value(item, &format!("t{i}"))?);
                    } else {
                        parts.push(self.value(item, "()")?);
                    }
                }
                format!(
                    "{{ let ({},) = {e}; ({},) }}",
                    names.join(", "),
                    parts.join(", ")
                )
            }
            Ty::Ref(inner, _) if matches!(**inner, Ty::Dyn(..)) => self.value(inner, e)?,
            Ty::Ref(inner, false) => format!("keep.val({})", self.value(inner, e)?),
            Ty::Ref(inner, true) => format!("keep.val_mut({})", self.value(inner, e)?),
            Ty::Boxed(inner) => format!("Box::new({})", self.value(inner, e)?),
            Ty::Named(def) => self.named(*def, e)?,
            _ => return Err(format!("can't lift {ty:?} in place")),
        })
    }

    fn named(&self, id: usize, e: &str) -> Result<String, String> {
        let def = &self.types.defs[id];
        let path = rust_path(def);
        let path = path.split('<').next().unwrap();
        Ok(match &def.kind {
            Kind::Record(fields) => {
                let mut parts = Vec::new();
                for field in fields {
                    let source = if crosses(self.types, &field.ty) {
                        format!("w.{}", bound_ident(&field.wit))
                    } else {
                        "()".into()
                    };
                    parts.push(format!(
                        "{}: {}",
                        field.rust,
                        self.value(&field.ty, &source)?
                    ));
                }
                format!("{{ let w = {e}; {path} {{ {} }} }}", parts.join(", "))
            }
            Kind::TupleRecord(items) => {
                let mut parts = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let source = match items.len() {
                        1 => "w".to_string(),
                        _ => format!("w.{i}"),
                    };
                    parts.push(self.value(item, &source)?);
                }
                format!("{{ let w = {e}; {path}({}) }}", parts.join(", "))
            }
            Kind::Variant(cases) => {
                // Only cases carrying a trait object borrow: the side enums.
                let mut arms = Vec::new();
                for Case { rust, wit, payload } in cases {
                    let Payload::Tuple(items) = payload else {
                        return Err(format!("{} borrows in a record case", def.rust));
                    };
                    let mut args = Vec::new();
                    for item in items {
                        match item {
                            Ty::Dyn(..) => args.push(self.value(item, "()")?),
                            _ => return Err(format!("{} borrows a value", def.rust)),
                        }
                    }
                    arms.push(format!(
                        "{}::{} => {path}::{rust}({})",
                        wire_path(def),
                        bound_type(wit),
                        args.join(", ")
                    ));
                }
                format!("match {e} {{ {} }}", arms.join(", "))
            }
            _ => return Err(format!("can't lift {} in place", def.rust)),
        })
    }
}

/// The conversion functions of every def the bound methods reach.
pub(super) fn conversions(
    types: &Types<'_>,
    defs: &std::collections::BTreeSet<usize>,
    outs: &std::collections::BTreeSet<usize>,
) -> String {
    let mut code = String::new();
    for &id in defs {
        let def = &types.defs[id];
        if types.refused[id].is_some() {
            continue;
        }
        let rust = rust_path(def);
        let wire = wire_path(def);
        // Lowering.
        let body = match &def.kind {
            Kind::Record(fields) => {
                let parts: Vec<String> = fields
                    .iter()
                    .filter(|f| crosses(types, &f.ty))
                    .map(|f| {
                        format!(
                            "{}: {}",
                            bound_ident(&f.wit),
                            lower(types, &f.ty, &format!("(&v.{})", f.rust))
                        )
                    })
                    .collect();
                if parts.is_empty() {
                    format!("{wire}::Value")
                } else {
                    format!("{wire} {{ {} }}", parts.join(", "))
                }
            }
            Kind::TupleRecord(items) if items.iter().all(|i| *i == Ty::Unit) => {
                format!("{wire}::Value")
            }
            Kind::TupleRecord(items) => match items.as_slice() {
                [one] => lower(types, one, "(&v.0)"),
                _ => {
                    let parts: Vec<String> = items
                        .iter()
                        .enumerate()
                        .map(|(i, item)| lower(types, item, &format!("(&v.{i})")))
                        .collect();
                    format!("({},)", parts.join(", "))
                }
            },
            Kind::Unit => format!("{wire}::Value"),
            Kind::Enum(cases) => {
                let path = rust.split('<').next().unwrap();
                let arms: Vec<String> = cases
                    .iter()
                    .map(|c| {
                        format!(
                            "{path}::{c} => {wire}::{}",
                            bound_type(&wit_ident(&c.to_snake_case()))
                        )
                    })
                    .collect();
                format!("match v {{ {}, {EVERY_CASE} }}", arms.join(", "))
            }
            Kind::Variant(cases) => {
                let path = rust.split('<').next().unwrap();
                let arms: Vec<String> = cases
                    .iter()
                    .map(
                        |Case {
                             rust: case,
                             wit: w,
                             payload,
                         }| {
                            let target = format!("{wire}::{}", bound_type(&wit_ident(w)));
                            match payload {
                                Payload::None => format!("{path}::{case} => {target}"),
                                Payload::Tuple(items) => {
                                    let names: Vec<String> =
                                        (0..items.len()).map(|i| format!("p{i}")).collect();
                                    let lowered: Vec<String> = items
                                        .iter()
                                        .zip(&names)
                                        .filter(|(item, _)| crosses(types, item))
                                        .map(|(item, name)| lower(types, item, name))
                                        .collect();
                                    let pattern = format!("{path}::{case}({})", names.join(", "));
                                    match lowered.len() {
                                        0 => format!(
                                            "{pattern} => {{ let _ = ({},); {target} }}",
                                            names.join(", ")
                                        ),
                                        1 => format!("{pattern} => {target}({})", lowered[0]),
                                        _ => format!(
                                            "{pattern} => {target}(({},))",
                                            lowered.join(", ")
                                        ),
                                    }
                                }
                                Payload::Record(record, fields) => {
                                    let names: Vec<&str> =
                                        fields.iter().map(|f| f.rust.as_str()).collect();
                                    let parts: Vec<String> = fields
                                        .iter()
                                        .filter(|f| crosses(types, &f.ty))
                                        .map(|f| {
                                            format!(
                                                "{}: {}",
                                                bound_ident(&f.wit),
                                                lower(types, &f.ty, &f.rust)
                                            )
                                        })
                                        .collect();
                                    format!(
                                        "{path}::{case} {{ {} }} => {target}({W}::{} {{ {} }})",
                                        names.join(", "),
                                        bound_type(record),
                                        parts.join(", ")
                                    )
                                }
                            }
                        },
                    )
                    .collect();
                format!("match v {{ {}, {EVERY_CASE} }}", arms.join(", "))
            }
            Kind::Via(form) => format!(
                "{{ let f: {} = ::core::convert::From::from(v); {} }}",
                super::sides::rust_ty(types, form, false),
                lower(types, form, "(&f)")
            ),
            Kind::Json => "serde_json::to_string(v).unwrap_or_default()".into(),
            Kind::Opaque(_) => unreachable!(),
        };
        writeln!(code, "#[allow(unused_variables, unreachable_patterns, clippy::all)] fn lower_{id}(v: &{rust}) -> {wire} {{ {body} }}").unwrap();
        // Lifting, for types that own their contents.
        if def.lifetimes == 0 {
            let path = rust.as_str();
            let body = match &def.kind {
                Kind::Record(fields) => {
                    let parts: Vec<String> = fields
                        .iter()
                        .map(|f| {
                            let value = if crosses(types, &f.ty) {
                                lift(types, &f.ty, &format!("w.{}", bound_ident(&f.wit)))
                            } else {
                                "Default::default()".into()
                            };
                            format!("{}: {value}", f.rust)
                        })
                        .collect();
                    format!("{path} {{ {} }}", parts.join(", "))
                }
                Kind::TupleRecord(items) if items.iter().all(|i| *i == Ty::Unit) => {
                    let units = vec!["()"; items.len()].join(", ");
                    format!("{{ let _ = w; {path}({units}) }}")
                }
                Kind::TupleRecord(items) => match items.as_slice() {
                    [one] => format!("{path}({})", lift(types, one, "w")),
                    _ => {
                        let names: Vec<String> =
                            (0..items.len()).map(|i| format!("t{i}")).collect();
                        let parts: Vec<String> = items
                            .iter()
                            .zip(&names)
                            .map(|(item, name)| lift(types, item, name))
                            .collect();
                        format!(
                            "{{ let ({},) = w; {path}({}) }}",
                            names.join(", "),
                            parts.join(", ")
                        )
                    }
                },
                Kind::Unit => format!("{{ let _ = w; {path} }}"),
                Kind::Enum(cases) => {
                    let arms: Vec<String> = cases
                        .iter()
                        .map(|c| {
                            format!(
                                "{wire}::{} => {path}::{c}",
                                bound_type(&wit_ident(&c.to_snake_case()))
                            )
                        })
                        .collect();
                    format!("match w {{ {} }}", arms.join(", "))
                }
                Kind::Variant(cases) => {
                    let arms: Vec<String> = cases
                        .iter()
                        .map(
                            |Case {
                                 rust: case,
                                 wit: w,
                                 payload,
                             }| {
                                let source = format!("{wire}::{}", bound_type(&wit_ident(w)));
                                match payload {
                                    Payload::None => format!("{source} => {path}::{case}"),
                                    Payload::Tuple(items) => {
                                        let crossing: Vec<&Ty> =
                                            items.iter().filter(|i| crosses(types, i)).collect();
                                        let values: Vec<String> = match crossing.len() {
                                            0 => Vec::new(),
                                            1 => vec![lift(types, crossing[0], "p")],
                                            _ => (0..crossing.len())
                                                .map(|i| {
                                                    lift(types, crossing[i], &format!("p.{i}"))
                                                })
                                                .collect(),
                                        };
                                        if crossing.is_empty() {
                                            format!("{source} => {path}::{case}()")
                                        } else {
                                            format!(
                                                "{source}(p) => {path}::{case}({})",
                                                values.join(", ")
                                            )
                                        }
                                    }
                                    Payload::Record(_, fields) => {
                                        let parts: Vec<String> = fields
                                            .iter()
                                            .map(|f| {
                                                format!(
                                                    "{}: {}",
                                                    f.rust,
                                                    lift(
                                                        types,
                                                        &f.ty,
                                                        &format!("p.{}", bound_ident(&f.wit))
                                                    )
                                                )
                                            })
                                            .collect();
                                        format!(
                                            "{source}(p) => {path}::{case} {{ {} }}",
                                            parts.join(", ")
                                        )
                                    }
                                }
                            },
                        )
                        .collect();
                    format!("match w {{ {} }}", arms.join(", "))
                }
                Kind::Via(form) => format!(
                    "{{ let f: {} = {}; ::core::convert::TryFrom::try_from(f).map_err(|_| Refused)? }}",
                    super::sides::rust_ty(types, form, false),
                    lift(types, form, "w")
                ),
                Kind::Json => "serde_json::from_str(&w).map_err(|_| Refused)?".into(),
                Kind::Opaque(_) => unreachable!(),
            };
            writeln!(code, "#[allow(unused_variables, clippy::all)] fn lift_{id}(w: {wire}) -> Result<{path}, Refused> {{ Ok({body}) }}").unwrap();
        }
        // An event's written-back fields, both ways.
        if outs.contains(&id)
            && let Kind::Record(fields) = &def.kind
        {
            let writable: Vec<_> = fields
                .iter()
                .filter(|f| writable(&f.ty) && crosses(types, &f.ty))
                .collect();
            if writable.is_empty() {
                continue;
            }
            let out = format!("{W}::{}", bound_type(&wit::out_name(def)));
            let parts: Vec<String> = writable
                .iter()
                .map(|f| {
                    let value = match &f.ty {
                        Ty::Ref(inner, true)
                            if let Ty::Named(inner) = &**inner
                                && wit::has_out(types, &types.defs[*inner]) =>
                        {
                            format!("lower_out_{inner}(&*v.{})", f.rust)
                        }
                        ty => lower(types, ty, &format!("(&v.{})", f.rust)),
                    };
                    format!("{}: {value}", bound_ident(&f.wit))
                })
                .collect();
            writeln!(code, "#[allow(unused_variables, clippy::all)] fn lower_out_{id}(v: &{rust}) -> {out} {{ {out} {{ {} }} }}", parts.join(", ")).unwrap();
            let mut writes = String::new();
            for f in &writable {
                let source = format!("o.{}", bound_ident(&f.wit));
                match &f.ty {
                    Ty::Ref(inner, true)
                        if let Ty::Named(inner) = &**inner
                            && wit::has_out(types, &types.defs[*inner]) =>
                    {
                        writeln!(writes, "write_back_{inner}(&mut *v.{}, {source})?;", f.rust)
                            .unwrap();
                    }
                    Ty::Ref(inner, true) if !borrows(types, inner) => {
                        writeln!(writes, "*v.{} = {};", f.rust, lift(types, inner, &source))
                            .unwrap()
                    }
                    Ty::Ref(inner, true) => {
                        if let Ty::Named(inner) = &**inner
                            && outs.contains(inner)
                        {
                            writeln!(writes, "write_back_{inner}(v.{}, {source})?;", f.rust)
                                .unwrap();
                        } else {
                            writeln!(writes, "let _ = {source};").unwrap();
                        }
                    }
                    ty if !borrows(types, ty) => {
                        writeln!(writes, "v.{} = {};", f.rust, lift(types, ty, &source)).unwrap()
                    }
                    // A wrapper around one mutable borrow (`CarriedAccess`): write through it.
                    Ty::Named(inner)
                        if let Kind::TupleRecord(items) = &types.defs[*inner].kind
                            && let [Ty::Ref(target, true)] = items.as_slice()
                            && !borrows(types, target) =>
                    {
                        writeln!(
                            writes,
                            "*v.{}.0 = {};",
                            f.rust,
                            lift(types, target, &source)
                        )
                        .unwrap()
                    }
                    _ => writeln!(writes, "let _ = {source};").unwrap(),
                }
            }
            writeln!(code, "#[allow(unused_variables, clippy::all)] fn write_back_{id}(v: &mut {rust}, o: {out}) -> Result<(), Refused> {{ {writes} Ok(()) }}").unwrap();
        }
    }
    code
}

/// Each method's results, as the bindings return them: one value, or a tuple.
pub(super) fn unpack(sig: &Signature) -> (String, Vec<String>) {
    let names: Vec<String> = (0..sig.results.len()).map(|i| format!("r{i}")).collect();
    let pattern = match names.len() {
        0 => "()".to_string(),
        1 => names[0].clone(),
        _ => format!("({})", names.join(", ")),
    };
    (pattern, names)
}
