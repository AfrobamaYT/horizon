//! Shared WIT world import registration emitter. The caller owns callback types and bodies;
//! Wasmtime checks those types against the component's WIT when a group is instantiated.
use std::fmt::Write;

/// A selected import's callback and lowering strategy.
pub struct Registration {
    /// Rust tuple pattern and its full type, including the empty tuple.
    pub parameters: String,
    /// A Rust expression returning `wasmtime::Result<(...)>`.
    pub body: String,
    /// Whether this replaces a binding already registered by bindgen.
    pub shadow: bool,
}

/// Emit registrations in WIT order. `select` receives the canonical interface name and WIT
/// function; `None` leaves that import untouched. It can emit a plain store-aware callback or
/// a `WasmList` callback with store-owned scratch using the same registration machinery.
pub fn emit(
    resolve: &wit_parser::Resolve,
    world: wit_parser::WorldId,
    mut select: impl FnMut(&str, &wit_parser::Function) -> Result<Option<Registration>, String>,
) -> Result<String, String> {
    let mut result = String::new();
    for (key, item) in &resolve.worlds[world].imports {
        let wit_parser::WorldItem::Interface { id, .. } = item else {
            continue;
        };
        let name = resolve.name_world_key(key);
        for function in resolve.interfaces[*id].functions.values() {
            let Some(registration) = select(&name, function)? else {
                continue;
            };
            if registration.shadow {
                result.push_str("linker.allow_shadowing(true);\n");
            }
            writeln!(result, "linker.instance({name:?})?.func_wrap({:?}, |mut store: wasmtime::StoreContextMut<'_, super::State>, {}| {{ {} }})?;", function.name, registration.parameters, registration.body).unwrap();
            if registration.shadow {
                result.push_str("linker.allow_shadowing(false);\n");
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_selected_functions_from_any_world() {
        for (package, world, interface) in [
            ("fixture:first@1.0.0", "alpha", "events"),
            ("fixture:second@2.1.0", "beta", "view"),
        ] {
            let text = format!(
                "package {package}; interface {interface} {{ read: func(n: u32) -> u32; skip: func(); }} world {world} {{ import {interface}; }}"
            );
            let mut resolve = wit_parser::Resolve::default();
            let package = resolve.push_str("fixture.wit", &text).unwrap();
            let world = resolve.select_world(&[package], Some(world)).unwrap();
            let generated = emit(&resolve, world, |_, function| {
                Ok((function.name == "read").then(|| Registration {
                    parameters: "(n,): (u32,)".into(),
                    body: "Ok((n,))".into(),
                    shadow: false,
                }))
            })
            .unwrap();
            assert!(generated.contains(".func_wrap(\"read\""));
            assert!(!generated.contains(".func_wrap(\"skip\""));
            assert!(!generated.contains("allow_shadowing"));
        }
    }
}
