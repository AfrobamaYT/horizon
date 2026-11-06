use heck::{ToSnakeCase, ToUpperCamelCase};
use horizon_mod_api_gen::wasm_imports::{Registration, emit};
use wit_parser::{Resolve, Type, TypeDefKind};

fn rust_type(resolve: &Resolve, ty: Type) -> Result<String, String> {
    Ok(match ty {
        Type::U32 => "u32".into(),
        Type::U64 => "u64".into(),
        Type::Id(id) => {
            let def = &resolve.types[id];
            match &def.kind {
                TypeDefKind::Handle(handle) => {
                    let id = match handle {
                        wit_parser::Handle::Own(id) | wit_parser::Handle::Borrow(id) => *id,
                    };
                    format!(
                        "Resource<wire::{}>",
                        resolve.types[id]
                            .name
                            .as_ref()
                            .unwrap()
                            .to_upper_camel_case()
                    )
                }
                TypeDefKind::Option(inner) => format!("Option<{}>", rust_type(resolve, *inner)?),
                TypeDefKind::List(inner) => format!("Vec<{}>", rust_type(resolve, *inner)?),
                TypeDefKind::Tuple(tuple) => format!(
                    "({},)",
                    tuple
                        .types
                        .iter()
                        .map(|ty| rust_type(resolve, *ty))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(",")
                ),
                TypeDefKind::Type(inner) => rust_type(resolve, *inner)?,
                _ => format!(
                    "wire::{}",
                    def.name
                        .as_ref()
                        .ok_or_else(|| format!("unnamed WIT type: {:?}", def.kind))?
                        .to_upper_camel_case()
                ),
            }
        }
        _ => return Err(format!("unsupported internals input type {ty:?}")),
    })
}

fn lazy_type(resolve: &Resolve, ty: Type, qualified: bool) -> Result<String, String> {
    Ok(match ty {
        Type::String => "StringInput".into(),
        Type::Bool => "bool".into(),
        Type::Char => "char".into(),
        Type::S64 => "i64".into(),
        Type::F64 => "f64".into(),
        Type::Id(id) => match &resolve.types[id].kind {
            TypeDefKind::Variant(_) => format!(
                "{}{}",
                if qualified { "lazy::" } else { "" },
                resolve.types[id]
                    .name
                    .as_ref()
                    .unwrap()
                    .to_upper_camel_case()
            ),
            TypeDefKind::List(inner) => format!(
                "wasmtime::component::WasmList<{}>",
                lazy_type(resolve, *inner, qualified)?
            ),
            TypeDefKind::Tuple(t) => format!(
                "({},)",
                t.types
                    .iter()
                    .map(|ty| lazy_type(resolve, *ty, qualified))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(",")
            ),
            TypeDefKind::Option(inner) => {
                format!("Option<{}>", lazy_type(resolve, *inner, qualified)?)
            }
            TypeDefKind::Type(inner) => lazy_type(resolve, *inner, qualified)?,
            _ => rust_type(resolve, ty)?,
        },
        _ => rust_type(resolve, ty)?,
    })
}

/// Generate cursor-bearing variants from WIT rather than maintaining a second
/// handwritten cell/argument schema. Only types reached by import parameters
/// are emitted; outputs keep the ordinary owned bindings.
fn lazy_variants(resolve: &Resolve, world: wit_parser::WorldId) -> Result<String, String> {
    fn visit(resolve: &Resolve, ty: Type, seen: &mut std::collections::BTreeSet<usize>) {
        let Type::Id(id) = ty else { return };
        if !seen.insert(id.index()) {
            return;
        }
        match &resolve.types[id].kind {
            TypeDefKind::Variant(v) => {
                for case in &v.cases {
                    if let Some(ty) = case.ty {
                        visit(resolve, ty, seen);
                    }
                }
            }
            TypeDefKind::Option(ty) | TypeDefKind::List(ty) | TypeDefKind::Type(ty) => {
                visit(resolve, *ty, seen)
            }
            TypeDefKind::Tuple(t) => {
                for ty in &t.types {
                    visit(resolve, *ty, seen);
                }
            }
            _ => {}
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for item in resolve.worlds[world].imports.values() {
        if let wit_parser::WorldItem::Interface { id, .. } = item {
            for f in resolve.interfaces[*id].functions.values() {
                for p in &f.params {
                    visit(resolve, p.ty, &mut seen);
                }
            }
        }
    }
    let mut output = String::new();
    for (id, def) in resolve.types.iter() {
        if !seen.contains(&id.index()) {
            continue;
        }
        let TypeDefKind::Variant(v) = &def.kind else {
            continue;
        };
        let name = def.name.as_ref().unwrap().to_upper_camel_case();
        output.push_str("#[derive(wasmtime::component::ComponentType, wasmtime::component::Lift)]\n#[component(variant)]\n");
        output.push_str(&format!("pub(super) enum {name} {{\n"));
        for case in &v.cases {
            let payload = case
                .ty
                .map(|ty| lazy_type(resolve, ty, false).map(|ty| format!("({ty})")))
                .transpose()?
                .unwrap_or_default();
            output.push_str(&format!(
                "#[component(name = {:?})] {}{payload},\n",
                case.name,
                case.name.to_upper_camel_case()
            ));
        }
        output.push_str("}\n");
        output.push_str(&format!("impl Materialize for {name} {{ type Owned = wire::{name}; fn materialize(self, input: &mut Input<'_, '_>) -> wasmtime::Result<Option<Self::Owned>> {{ Ok(Some(match self {{\n"));
        for case in &v.cases {
            let case_name = case.name.to_upper_camel_case();
            if case.ty.is_some() {
                output.push_str(&format!("Self::{case_name}(v) => wire::{name}::{case_name}(match v.materialize(input)? {{ Some(v) => v, None => return Ok(None) }}),\n"));
            } else {
                output.push_str(&format!(
                    "Self::{case_name} => wire::{name}::{case_name},\n"
                ));
            }
        }
        output.push_str("} )) } }\n");
    }
    Ok(output)
}

fn main() {
    let path = "../horizon-mod-points/wit/internals.wit";
    println!("cargo:rerun-if-changed={path}");
    let mut resolve = Resolve::default();
    let package = resolve
        .push_str(path, &std::fs::read_to_string(path).unwrap())
        .unwrap();
    let world = resolve.select_world(&[package], Some("internals")).unwrap();
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("internals_lazy.rs"),
        lazy_variants(&resolve, world).unwrap(),
    )
    .unwrap();
    let output = emit(
        &resolve,
        world,
        |interface| {
            if interface != "horizon:mod-internals/view@0.1.0" {
                return Err(format!("unhandled internals interface {interface}"));
            }
            Ok(())
        },
        |_, function| {
            let name = function
                .name
                .rsplit_once('.')
                .map_or(function.name.as_str(), |(_, n)| n)
                .to_snake_case();
            let args: Vec<_> = (0..function.params.len())
                .map(|i| format!("arg{i}"))
                .collect();
            let lazy = function.params.iter().any(|p| matches!(p.ty, Type::Id(id) if matches!(resolve.types[id].kind, TypeDefKind::List(_))))
                && matches!(function.result, Some(Type::Id(id)) if matches!(resolve.types[id].kind, TypeDefKind::Result(_)));
            let types = function
                .params
                .iter()
                .map(|p| if lazy { lazy_type(&resolve, p.ty, true) } else { rust_type(&resolve, p.ty) })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Some(Registration {
                parameters: format!("({},): ({},)", args.join(","), types.join(",")),
                body: format!(
                    "Ok((Metered::{name}(store.as_context_mut(), {})?,))",
                    args.join(",")
                ),
                shadow: false,
            }))
        },
    )
    .unwrap();
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("internals_imports.rs"),
        format!("{{ {output} }}"),
    )
    .unwrap();
}
