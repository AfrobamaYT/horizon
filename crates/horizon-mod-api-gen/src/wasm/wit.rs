//! The WIT text of the mod API: one `types` interface, an interface per bound trait, and the
//! world `mod` that imports the traits a mod calls and exports the ones it implements.
use super::ty::{Case, Def, Kind, Payload, Ty, Types, wit_ident};
use super::{EXPORTS, Method};
use heck::ToKebabCase;
use std::collections::BTreeSet;
use std::fmt::Write;

/// The WIT type of `ty`, or `None` when no value crosses for it (a host or context reached
/// through the call's frame, or `()`).
pub(super) fn wit(types: &Types<'_>, ty: &Ty) -> Result<Option<String>, String> {
    Ok(Some(match ty {
        Ty::Unit | Ty::Dyn(..) => return Ok(None),
        Ty::Prim(_, wit) => (*wit).into(),
        Ty::Wide(_) => "tuple<u64, u64>".into(),
        Ty::Str(_) => "string".into(),
        Ty::List(item, _) | Ty::Array(item, _) => format!("list<{}>", value(types, item)?),
        Ty::Option(item) => format!("option<{}>", value(types, item)?),
        Ty::Result(ok, err) => match (wit(types, ok)?, wit(types, err)?) {
            (None, None) => "result".into(),
            (Some(ok), None) => format!("result<{ok}>"),
            (None, Some(err)) => format!("result<_, {err}>"),
            (Some(ok), Some(err)) => format!("result<{ok}, {err}>"),
        },
        Ty::Tuple(items) => {
            let items = items
                .iter()
                .map(|item| wit(types, item))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            match items.len() {
                0 => return Ok(None),
                _ => format!("tuple<{}>", items.join(", ")),
            }
        }
        Ty::Ref(inner, _) | Ty::Boxed(inner) => return wit(types, inner),
        Ty::Named(id) => {
            if let Some(why) = types.refused.get(*id).and_then(Option::as_ref) {
                return Err(why.clone());
            }
            types.defs[*id].wit.clone()
        }
    }))
}

/// Marks each def that can't cross, and why: an opaque one, or one with a field or payload
/// that can't, found again until nothing changes.
pub(super) fn refuse(types: &mut Types<'_>) {
    types.refused = types
        .defs
        .iter()
        .map(|def| match &def.kind {
            Kind::Opaque(why) => Some(format!("{}: {why}", def.rust)),
            _ => None,
        })
        .collect();
    // WIT has no recursive types.
    for id in 0..types.defs.len() {
        let mut reached = BTreeSet::new();
        for ty in def_fields(&types.defs[id]) {
            named(types, ty, &mut reached);
        }
        if reached.contains(&id) && types.refused[id].is_none() {
            types.refused[id] = Some(format!("{} contains itself", types.defs[id].rust));
        }
    }
    loop {
        let mut changed = false;
        for id in 0..types.defs.len() {
            if types.refused[id].is_some() {
                continue;
            }
            let def = &types.defs[id];
            let why = def_fields(def).into_iter().find_map(|ty| {
                let checked = match ty {
                    Ty::Dyn(..) => Ok(None),
                    ty => wit(types, ty),
                };
                checked.err()
            });
            if let Some(why) = why {
                let why = format!("{} holds {why}", def.rust);
                types.refused[id] = Some(why);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

/// The WIT type of a value nested in another, which must cross.
fn value(types: &Types<'_>, ty: &Ty) -> Result<String, String> {
    wit(types, ty)?.ok_or_else(|| format!("{ty:?} can't cross inside a value"))
}

/// The defs a type names, directly or through other defs.
pub(super) fn named(types: &Types<'_>, ty: &Ty, out: &mut BTreeSet<usize>) {
    match ty {
        Ty::List(item, _)
        | Ty::Array(item, _)
        | Ty::Option(item)
        | Ty::Ref(item, _)
        | Ty::Boxed(item) => named(types, item, out),
        Ty::Result(a, b) => {
            named(types, a, out);
            named(types, b, out);
        }
        Ty::Tuple(items) => items.iter().for_each(|item| named(types, item, out)),
        Ty::Named(def) => {
            if out.insert(*def) {
                for ty in def_fields(&types.defs[*def]) {
                    named(types, ty, out);
                }
            }
        }
        _ => {}
    }
}

/// Every type a def's fields and payloads name.
pub(super) fn def_fields(def: &Def) -> Vec<&Ty> {
    match &def.kind {
        Kind::Record(fields) => fields.iter().map(|f| &f.ty).collect(),
        Kind::TupleRecord(items) => items.iter().collect(),
        Kind::Variant(cases) => cases
            .iter()
            .flat_map(|case| match &case.payload {
                Payload::None => Vec::new(),
                Payload::Tuple(items) => items.iter().collect(),
                Payload::Record(_, fields) => fields.iter().map(|f| &f.ty).collect(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether a field of an event passed `&mut` is written back: everything the mod owns a copy
/// of, and what it was lent mutably.
pub(super) fn writable(ty: &Ty) -> bool {
    match ty {
        Ty::Ref(_, mutable) | Ty::Dyn(_, mutable) => *mutable,
        Ty::Str(super::ty::Str::Borrowed) => false,
        Ty::List(_, super::ty::List::Slice) => false,
        Ty::Option(inner) => !matches!(**inner, Ty::Ref(_, false)) && writable_inner(inner),
        _ => true,
    }
}

fn writable_inner(ty: &Ty) -> bool {
    !matches!(ty, Ty::Dyn(..))
}

/// The WIT name of the record carrying `def`'s written-back fields.
pub(super) fn out_name(def: &Def) -> String {
    format!("{}-out", def.wit)
}

/// Whether `def` is a record with fields a `&mut` borrow writes back: it then has an `-out`.
pub(super) fn has_out(types: &Types<'_>, def: &Def) -> bool {
    let Kind::Record(fields) = &def.kind else {
        return false;
    };
    fields
        .iter()
        .any(|f| writable(&f.ty) && wit(types, &f.ty).ok().flatten().is_some())
}

/// The WIT of one def.
fn def_wit(types: &Types<'_>, def: &Def, out: &mut String, used_out: bool) -> Result<(), String> {
    let name = &def.wit;
    let fields = |fields: &[super::ty::Field]| -> Result<String, String> {
        let mut text = String::new();
        for field in fields {
            if let Some(ty) = wit(types, &field.ty)? {
                write!(text, "{}: {ty}, ", field.wit).unwrap();
            }
        }
        Ok(text)
    };
    match &def.kind {
        Kind::Record(list) => {
            let text = fields(list)?;
            if text.is_empty() {
                writeln!(out, "  enum {name} {{ value }}").unwrap();
            } else {
                writeln!(out, "  record {name} {{ {text}}}").unwrap();
            }
            if used_out {
                let writable: Vec<_> = list
                    .iter()
                    .filter(|f| writable(&f.ty))
                    .map(|f| super::ty::Field {
                        rust: f.rust.clone(),
                        wit: f.wit.clone(),
                        ty: f.ty.clone(),
                    })
                    .collect();
                // A `&mut` record inside is written back too, as its own `-out`.
                let mut text = String::new();
                for field in &writable {
                    match &field.ty {
                        Ty::Ref(inner, true)
                            if let Ty::Named(id) = &**inner
                                && has_out(types, &types.defs[*id]) =>
                        {
                            write!(text, "{}: {}, ", field.wit, out_name(&types.defs[*id]))
                                .unwrap();
                        }
                        ty => {
                            if let Some(ty) = wit(types, ty)? {
                                write!(text, "{}: {ty}, ", field.wit).unwrap();
                            }
                        }
                    }
                }
                if !text.is_empty() {
                    writeln!(out, "  record {} {{ {text}}}", out_name(def)).unwrap();
                }
            }
        }
        Kind::TupleRecord(items) => {
            let ty = wit(types, &Ty::Tuple(items.clone()))?.ok_or("empty tuple struct")?;
            let ty = match items.as_slice() {
                [one] => value(types, one)?,
                _ => ty,
            };
            writeln!(out, "  type {name} = {ty};").unwrap();
        }
        Kind::Unit => writeln!(out, "  enum {name} {{ value }}").unwrap(),
        Kind::Enum(cases) => {
            let cases: Vec<_> = cases
                .iter()
                .map(|c| wit_ident(&c.to_kebab_case()))
                .collect();
            writeln!(out, "  enum {name} {{ {} }}", cases.join(", ")).unwrap();
        }
        Kind::Variant(cases) => {
            let mut text = Vec::new();
            for Case {
                wit: case, payload, ..
            } in cases
            {
                let case = wit_ident(case);
                match payload {
                    Payload::None => text.push(case),
                    Payload::Tuple(items) => match items.as_slice() {
                        [one] => match wit(types, one)? {
                            Some(ty) => text.push(format!("{case}({ty})")),
                            None => text.push(case),
                        },
                        _ => match wit(types, &Ty::Tuple(items.clone()))? {
                            Some(ty) => text.push(format!("{case}({ty})")),
                            None => text.push(case),
                        },
                    },
                    Payload::Record(record, list) => {
                        writeln!(out, "  record {record} {{ {}}}", fields(list)?).unwrap();
                        text.push(format!("{case}({record})"));
                    }
                }
            }
            writeln!(out, "  variant {name} {{ {} }}", text.join(", ")).unwrap();
        }
        Kind::Codec(wire) if wire == name => {}
        Kind::Codec(wire) => writeln!(out, "  type {name} = {wire};").unwrap(),
        Kind::Json => writeln!(out, "  type {name} = string;").unwrap(),
        Kind::Opaque(why) => return Err(format!("{}: {why}", def.rust)),
    }
    Ok(())
}

/// One method's WIT function: its arguments and what it returns.
pub(super) struct Signature {
    pub(super) params: Vec<(String, String)>,
    pub(super) results: Vec<String>,
}

/// The method's crossing shape: the handler index first for an exported part or handler,
/// each value argument, and back the result plus each `&mut` argument's written-back form.
pub(super) fn signature(types: &Types<'_>, method: &Method) -> Result<Signature, String> {
    let mut params = Vec::new();
    if EXPORTS.contains(&method.owner) && method.owner != "Hooks" {
        params.push(("handler".to_string(), "u32".to_string()));
    }
    let mut results = Vec::new();
    if let Some(ty) = wit(types, &method.ret)? {
        results.push(ty);
    }
    for (name, ty) in &method.params {
        if let Some(wire) = wit(types, ty)? {
            params.push((wit_ident(name), wire));
        }
        if let Ty::Ref(inner, true) = ty {
            match &**inner {
                Ty::Named(def) if matches!(types.defs[*def].kind, Kind::Record(_)) => {
                    let def = &types.defs[*def];
                    let Kind::Record(fields) = &def.kind else {
                        unreachable!()
                    };
                    if fields
                        .iter()
                        .any(|f| writable(&f.ty) && wit(types, &f.ty).ok().flatten().is_some())
                    {
                        results.push(out_name(def));
                    }
                }
                inner => {
                    if let Some(ty) = wit(types, inner)? {
                        results.push(ty);
                    }
                }
            }
        }
    }
    Ok(Signature { params, results })
}

/// The interface a trait's methods live in.
pub(super) fn interface(owner: &str) -> String {
    owner.rsplit("::").next().unwrap().to_kebab_case()
}

/// Every def the methods reach, and those passed `&mut` (with written-back records).
pub(super) fn reached(
    types: &Types<'_>,
    methods: &[&Method],
) -> (BTreeSet<usize>, BTreeSet<usize>) {
    let mut defs = BTreeSet::new();
    let mut outs = BTreeSet::new();
    for method in methods {
        for (_, ty) in &method.params {
            named(types, ty, &mut defs);
            mut_records(ty, &mut outs);
        }
        named(types, &method.ret, &mut defs);
    }
    // A `&mut` record inside a written-back record is written back too.
    loop {
        let before = outs.len();
        for id in outs.clone() {
            for ty in def_fields(&types.defs[id]) {
                mut_records(ty, &mut outs);
            }
        }
        if outs.len() == before {
            break;
        }
    }
    (defs, outs)
}

fn mut_records(ty: &Ty, outs: &mut BTreeSet<usize>) {
    if let Ty::Ref(inner, true) = ty
        && let Ty::Named(def) = &**inner
    {
        outs.insert(*def);
    }
}

/// The whole WIT package.
pub(super) fn package(
    types: &Types<'_>,
    methods: &[&Method],
    version: &str,
) -> Result<String, String> {
    let (defs, outs) = reached(types, methods);
    let mut text = format!(
        "// @generated by horizon-mod-api-gen from the mod API's traits; do not edit.\npackage horizon:mod-api@{version};\n\ninterface types {{\n  record entity-ref {{ epoch: u64, id: u32 }}\n  record player-ref {{ entity: entity-ref, uuid: tuple<u64, u64> }}\n  record level-ref {{ epoch: u64, dimension: string }}\n  record chunk-ref {{ level: level-ref, pos: tuple<s32, s32> }}\n  record block-entity-ref {{ level: level-ref, pos: tuple<s32, s32, s32> }}\n  record boss-bar-ref {{ epoch: u64, id: tuple<u64, u64> }}\n  record raid-ref {{ level: level-ref, id: s32 }}\n  record menu-ref {{ player: player-ref, window: s32, generation: u64 }}\n  record global-pos {{ dimension: string, pos: tuple<s32, s32, s32> }}\n  variant config-file-error {{ no-calling-mod, not-owned, invalid-path, too-large, not-found, not-a-file, not-a-folder, symlink, io(string) }}\n",
    );
    let mut type_names = Vec::new();
    for def in &defs {
        let def = &types.defs[*def];
        def_wit(
            types,
            def,
            &mut text,
            outs.contains(
                &types
                    .defs
                    .iter()
                    .position(|d| std::ptr::eq(d, def))
                    .unwrap(),
            ),
        )?;
        type_names.push(def.wit.clone());
    }
    text.push_str("}\n");
    let mut owners: Vec<&str> = Vec::new();
    for method in methods {
        if !owners.contains(&method.owner) {
            owners.push(method.owner);
        }
    }
    let mut world = String::from("\nworld mod {\n");
    for owner in &owners {
        let name = interface(owner);
        let functions: Vec<String> = methods
            .iter()
            .filter(|m| m.owner == *owner)
            .map(|m| wit_ident(&m.name))
            .collect();
        // A type named like one of the interface's functions is used under another name.
        let renamed = |ty: &str| {
            if functions.iter().any(|f| f == ty) {
                format!("{ty}-type")
            } else {
                ty.to_string()
            }
        };
        let uses: Vec<String> = used(types, methods, owner)
            .into_iter()
            .map(|ty| match renamed(&ty) {
                alias if alias != ty => format!("{ty} as {alias}"),
                _ => ty,
            })
            .collect();
        writeln!(
            text,
            "\ninterface {name} {{\n  use types.{{{}}};",
            uses.join(", ")
        )
        .unwrap();
        let rename = |wire: &str| -> String {
            let mut out = String::new();
            let mut word = String::new();
            for c in wire.chars().chain(std::iter::once(' ')) {
                if c.is_ascii_alphanumeric() || c == '-' || c == '%' {
                    word.push(c);
                } else {
                    out.push_str(&renamed(&word));
                    word.clear();
                    if c != ' ' || !out.is_empty() {
                        out.push(c);
                    }
                }
            }
            out.trim_end().to_string()
        };
        for method in methods.iter().filter(|m| m.owner == *owner) {
            let sig = signature(types, method)?;
            let params: Vec<_> = sig
                .params
                .iter()
                .map(|(n, t)| format!("{n}: {}", rename(t)))
                .collect();
            let results: Vec<_> = sig.results.iter().map(|t| rename(t)).collect();
            let results = match results.as_slice() {
                [] => String::new(),
                [one] => format!(" -> {one}"),
                many => format!(" -> tuple<{}>", many.join(", ")),
            };
            writeln!(
                text,
                "  {}: func({}){results};",
                wit_ident(&method.name),
                params.join(", ")
            )
            .unwrap();
        }
        if let Some((_, _)) = super::SUBSCRIBED.iter().find(|(o, _)| o == owner) {
            let handler = if *owner == "Hooks" {
                ""
            } else {
                "handler: u32"
            };
            writeln!(text, "  subscribed: func({handler}) -> list<string>;").unwrap();
        }
        text.push_str("}\n");
        let way = if EXPORTS.contains(owner) {
            "export"
        } else {
            "import"
        };
        writeln!(world, "  {way} {name};").unwrap();
    }
    world.push_str("}\n");
    text.push_str(&world);
    Ok(text)
}

/// The type names a trait's interface uses.
fn used(types: &Types<'_>, methods: &[&Method], owner: &str) -> Vec<String> {
    let mut defs = BTreeSet::new();
    let mut outs = BTreeSet::new();
    for method in methods.iter().filter(|m| m.owner == owner) {
        for (_, ty) in &method.params {
            direct(ty, &mut defs);
            if let Ty::Ref(inner, true) = ty
                && let Ty::Named(def) = &**inner
                && signature(types, method)
                    .map(|sig| sig.results.contains(&out_name(&types.defs[*def])))
                    .unwrap_or(false)
            {
                outs.insert(out_name(&types.defs[*def]));
            }
        }
        direct(&method.ret, &mut defs);
    }
    let mut names: Vec<String> = defs
        .into_iter()
        .filter(|def| !matches!(types.defs[*def].kind, Kind::Opaque(_)))
        .map(|def| types.defs[def].wit.clone())
        .collect();
    names.extend(outs);
    names.sort();
    names.dedup();
    if names.is_empty() {
        // `use` needs a name; every interface may use the handle records.
        names.push("entity-ref".into());
    }
    names
}

/// The defs a type names at its top level, not through other defs.
fn direct(ty: &Ty, out: &mut BTreeSet<usize>) {
    match ty {
        Ty::List(item, _)
        | Ty::Array(item, _)
        | Ty::Option(item)
        | Ty::Ref(item, _)
        | Ty::Boxed(item) => direct(item, out),
        Ty::Result(a, b) => {
            direct(a, out);
            direct(b, out);
        }
        Ty::Tuple(items) => items.iter().for_each(|item| direct(item, out)),
        Ty::Named(def) => {
            out.insert(*def);
        }
        _ => {}
    }
}
