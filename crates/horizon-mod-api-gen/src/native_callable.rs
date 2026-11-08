//! Generate ordinary Wire codec closures for a native callable source.
use crate::{
    crossing::{Rule, Shape},
    native_value,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

/// Expressions accepted by `FunctionSource::new`: decode an argument tuple and encode its
/// result. The source checks for trailing data before invoking the native callable. Unsupported
/// non-owned or inaccessible shapes are reported by the same codec rules as native values.
pub struct CallableCodecs {
    /// Capture-free argument decoder expression.
    pub decode: TokenStream,
    /// Capture-free result encoder expression.
    pub encode: TokenStream,
}

/// Generate codecs for an ordered list of native parameters and one native result.
pub fn codecs(rule: &Rule, arguments: &[Shape], result: &Shape) -> Result<CallableCodecs, String> {
    let args = native_value::codec(rule, &Shape::Tuple(arguments.to_vec()))?;
    let result = native_value::codec(rule, result)?;
    let decode = args.decode;
    let encode = result.encode;
    Ok(CallableCodecs {
        decode: quote!(|input: &mut mod_api::wire::Reader<'_>| -> ::core::result::Result<_, mod_api::wire::WireError> { Ok(#decode) }),
        encode: quote!(|value, out: &mut mod_api::wire::Writer| { #encode }),
    })
}

/// The signature of a callback argument: `&dyn Fn(A..) -> R`, `&mut dyn FnMut(A..) -> R`, or
/// `fn(A..) -> R`, with any lifetimes. `None` for every other type.
fn signature(ty: &Type) -> Option<(Vec<Type>, Type)> {
    fn tuple(
        inputs: impl Iterator<Item = Type>,
        output: &syn::ReturnType,
    ) -> Option<(Vec<Type>, Type)> {
        let output = match output {
            syn::ReturnType::Default => syn::parse_quote!(()),
            syn::ReturnType::Type(_, ty) => (**ty).clone(),
        };
        Some((inputs.collect(), output))
    }
    match ty {
        Type::Paren(inner) => signature(&inner.elem),
        Type::Reference(reference) => signature(&reference.elem),
        Type::BareFn(function) => tuple(
            function.inputs.iter().map(|input| input.ty.clone()),
            &function.output,
        ),
        Type::TraitObject(object) => object.bounds.iter().find_map(|bound| {
            let syn::TypeParamBound::Trait(bound) = bound else {
                return None;
            };
            let last = bound.path.segments.last()?;
            let syn::PathArguments::Parenthesized(call) = &last.arguments else {
                return None;
            };
            matches!(last.ident.to_string().as_str(), "Fn" | "FnMut" | "FnOnce")
                .then(|| tuple(call.inputs.iter().cloned(), &call.output))?
        }),
        _ => None,
    }
}

/// Whether `ty` is a callback argument (a closure or function pointer).
pub fn is_callback(ty: &Type) -> bool {
    signature(ty).is_some()
}

/// One decoded argument: its owned type, and whether the callee takes it by shared reference.
fn argument(ty: &Type) -> Result<(Type, bool), String> {
    match ty {
        Type::Reference(reference) if reference.mutability.is_some() => Err(format!(
            "`{}` is a mutable reference: a value decoded from bytes cannot carry the callee's writes back",
            quote!(#ty)
        )),
        Type::Reference(reference) => Ok((
            match &*reference.elem {
                Type::Path(path) if path.path.is_ident("str") => syn::parse_quote!(String),
                elem => elem.clone(),
            },
            true,
        )),
        ty => Ok((ty.clone(), false)),
    }
}

/// The body of an invocation closure `|__roots, __bytes| -> Result<Vec<u8>, RowError>` for a
/// callback argument, decoding the argument tuple, calling `callee` (a place expression of the
/// callback's own type) and encoding its result, or the reason this signature cannot project.
/// Arguments and result are owned ordinary Wire values: a `&T` argument is decoded as `T` and
/// lent to the call, a `&mut T` argument or any layout without a Wire codec is a gap.
pub fn invocation(
    rule: &Rule,
    module: &str,
    owner: Option<&str>,
    ty: &Type,
    callee: TokenStream,
) -> Result<TokenStream, String> {
    let (inputs, output) = signature(ty).ok_or("not a callback")?;
    let mut shapes = Vec::new();
    let mut lend = Vec::new();
    let mut owned_types = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let (owned, by_reference) = argument(input)?;
        shapes.push(rule.shape(module, owner, &owned));
        owned_types.push(owned);
        let member = syn::Index::from(index);
        lend.push(if by_reference {
            quote!(&__arguments.#member)
        } else {
            quote!(__arguments.#member)
        });
    }
    let args = native_value::codec(rule, &Shape::Tuple(shapes))
        .map_err(|reason| format!("argument tuple: {reason}"))?;
    let result = native_value::codec(rule, &rule.shape(module, owner, &output))
        .map_err(|reason| format!("result `{}`: {reason}", quote!(#output)))?;
    let decode = args.decode;
    let encode = result.encode;
    Ok(quote! {
        let mut __reader = mod_api::wire::Reader::new(__bytes);
        let __arguments: (#(#owned_types,)*) = { let input = &mut __reader; #decode };
        if !__reader.rest().is_empty() {
            return Err(mod_api::wire::WireError::Trailing.into());
        }
        let __result = (#callee)(#(#lend),*);
        let mut __writer = mod_api::wire::Writer::new();
        { let out = &mut __writer; let value = &__result; #encode }
        if let Some(error) = __writer.error() {
            return Err(error.into());
        }
        Ok(__writer.into_bytes())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason(ty: &str) -> Result<(), String> {
        let rule = Rule::new(&[crate::internals::Internal {
            path: "horizon_client::BlockPos".into(),
            kind: "struct",
            signature: "pub struct BlockPos { pub x: i32, pub y: i32, pub z: i32 }".into(),
            cfg: String::new(),
            hash: String::new(),
            shape_only: false,
            opaque_reason: None,
            refs: Default::default(),
        }]);
        invocation(
            &rule,
            "horizon_client",
            None,
            &syn::parse_str(ty).unwrap(),
            quote!(callee),
        )
        .map(|_| ())
    }

    #[test]
    fn catalog_callbacks_project_or_name_their_gap() {
        for ty in [
            "&'s dyn Fn(BlockPos) -> bool",
            "&dyn Fn(&str) -> bool",
            "&dyn Fn(&str) -> Option<String>",
            "&mut dyn FnMut(u32, &str)",
        ] {
            assert_eq!(reason(ty), Ok(()), "{ty}");
        }
        let gap = |ty| reason(ty).unwrap_err();
        assert!(gap("fn(&mut Self, PlayerRef)").contains("mutable reference"));
        assert!(gap("&mut dyn FnMut(UiResources, &std::path::Path)").contains("argument tuple"));
        assert!(gap("&dyn Fn(&str) -> Option<Json>").contains("result"));
        assert!(!is_callback(&syn::parse_str("&dyn EventLoopCtl").unwrap()));
    }
}
