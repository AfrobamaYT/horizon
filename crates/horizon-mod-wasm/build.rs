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

fn main() {
    let path = "../horizon-mod-points/wit/internals.wit";
    println!("cargo:rerun-if-changed={path}");
    let mut resolve = Resolve::default();
    let package = resolve
        .push_str(path, &std::fs::read_to_string(path).unwrap())
        .unwrap();
    let world = resolve.select_world(&[package], Some("internals")).unwrap();
    let output = emit(&resolve, world, |interface, function| {
        if interface != "horizon:mod-internals/view@0.1.0" {
            return Err(format!("unhandled internals interface {interface}"));
        }
        let name = function
            .name
            .rsplit_once('.')
            .map_or(function.name.as_str(), |(_, n)| n)
            .to_snake_case();
        let args: Vec<_> = (0..function.params.len())
            .map(|i| format!("arg{i}"))
            .collect();
        let types = function
            .params
            .iter()
            .map(|p| rust_type(&resolve, p.ty))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(Registration {
            parameters: format!("({},): ({},)", args.join(","), types.join(",")),
            body: format!(
                "Ok((Metered::{name}(store.as_context_mut(), {})?,))",
                args.join(",")
            ),
            shadow: false,
        }))
    })
    .unwrap();
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("internals_imports.rs"),
        format!("{{ {output} }}"),
    )
    .unwrap();
}
