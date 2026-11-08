//! Generation of owned native values from ordinary Wire values. Unsupported layouts are
//! reported rather than guessed; generated decoders never borrow the packet.
use crate::crossing::{Collection, Rule, Scalar, Shape, Wrapper};
use proc_macro2::TokenStream;
use quote::quote;

/// A codec expression pair. `decode` expects `input: &mut mod_api::wire::Reader`;
/// `encode` expects `value` to be a shared reference and `out: &mut Writer`.
pub struct ValueCodec {
    pub decode: TokenStream,
    pub encode: TokenStream,
}

/// Generate a codec for one complete owned value. Recursive and inaccessible layouts are gaps.
pub fn codec(rule: &Rule, shape: &Shape) -> Result<ValueCodec, String> {
    build(rule, shape, &mut Vec::new())
}

fn build(rule: &Rule, shape: &Shape, stack: &mut Vec<String>) -> Result<ValueCodec, String> {
    let pair = match shape {
        Shape::Scalar(Scalar::Str) | Shape::Ref { .. } | Shape::Slice(_) | Shape::Opaque(_) => {
            return Err(format!("non-owned or opaque shape: {shape:?}"));
        }
        Shape::Scalar(_) | Shape::Unit => ValueCodec {
            decode: quote!(input.take()?),
            encode: quote!(out.put(value);),
        },
        Shape::Wrapped(Wrapper::Box, inner) => {
            let c = build(rule, inner, stack)?;
            let d = c.decode;
            let e = c.encode;
            ValueCodec {
                decode: quote!(Box::new(input.nested(|input| Ok(#d))?)),
                encode: quote!({ let value = &**value; #e }),
            }
        }
        Shape::Option(inner) => {
            let c = build(rule, inner, stack)?;
            let d = c.decode;
            let e = c.encode;
            ValueCodec {
                decode: quote!(match input.take::<u8>()? { 0 => None, 1 => Some(input.nested(|input| Ok(#d))?), tag => return Err(mod_api::wire::WireError::Tag(tag.into())) }),
                encode: quote!(match value { None => out.put(&0u8), Some(value) => { out.put(&1u8); #e } }),
            }
        }
        Shape::Tuple(parts) => {
            let cs = parts
                .iter()
                .map(|s| build(rule, s, stack))
                .collect::<Result<Vec<_>, _>>()?;
            let ds = cs.iter().map(|c| &c.decode);
            let es = cs.iter().enumerate().map(|(i, c)| {
                let i = syn::Index::from(i);
                let e = &c.encode;
                quote!({ let value = &value.#i; #e })
            });
            ValueCodec {
                decode: quote!(input.nested(|input| Ok((#(#ds,)*)))?),
                encode: quote!(#(#es)*),
            }
        }
        Shape::Array(inner, len) => {
            let c = build(rule, inner, stack)?;
            let d = c.decode;
            let e = c.encode;
            ValueCodec {
                decode: quote!(input.nested(|input| { let mut values = Vec::new(); for _ in 0..#len { values.push(#d); } values.try_into().map_err(|_| mod_api::wire::WireError::Length) })?),
                encode: quote!(for value in value.iter() { #e }),
            }
        }
        Shape::Collection(kind, parts)
            if parts.len() == 1
                && matches!(
                    kind,
                    Collection::Vec
                        | Collection::VecDeque
                        | Collection::HashSet
                        | Collection::BTreeSet
                ) =>
        {
            let c = build(rule, &parts[0], stack)?;
            let d = c.decode;
            let e = c.encode;
            ValueCodec {
                decode: quote!(input.nested(|input| { let len = input.take_len(1)?; let mut values = Vec::with_capacity(len); for _ in 0..len { values.push(#d); } Ok(values.into_iter().collect()) })?),
                encode: quote!({ out.put_len(value.len()); for value in value.iter() { #e } }),
            }
        }
        Shape::Collection(kind, parts)
            if parts.len() == 2 && matches!(kind, Collection::HashMap | Collection::BTreeMap) =>
        {
            let key = build(rule, &parts[0], stack)?;
            let val = build(rule, &parts[1], stack)?;
            let kd = key.decode;
            let vd = val.decode;
            let ke = key.encode;
            let ve = val.encode;
            ValueCodec {
                decode: quote!(input.nested(|input| { let len = input.take_len(1)?; let mut values = Vec::with_capacity(len); for _ in 0..len { values.push((#kd, #vd)); } Ok(values.into_iter().collect()) })?),
                encode: quote!({ out.put_len(value.len()); for (key, val) in value.iter() { { let value = key; #ke } { let value = val; #ve } } }),
            }
        }
        Shape::Named(path, _) => {
            if stack.contains(path) {
                return Err(format!("recursive layout: {path}"));
            }
            let named = rule
                .named(path)
                .ok_or_else(|| format!("unknown layout: {path}"))?;
            if named.lifetimes != 0
                || named.generic
                || named.guarded
                || named.opaque_reason.is_some()
                || (named.foreign && named.sealed)
            {
                return Err(format!("inaccessible layout: {path}"));
            }
            let name = path
                .strip_prefix("horizon_client::")
                .map_or_else(|| path.to_owned(), |p| format!("crate::{p}"));
            let ty: syn::Path =
                syn::parse_str(&name).map_err(|_| format!("invalid path: {path}"))?;
            stack.push(path.clone());
            let result = match &named.item {
                syn::Item::Struct(item) => fields(rule, named, path, &ty, &item.fields, stack),
                syn::Item::Enum(item) => {
                    let mut decoders = Vec::new();
                    let mut encoders = Vec::new();
                    for (index, variant) in item.variants.iter().enumerate() {
                        let variant_name = &variant.ident;
                        let variant_path: syn::Path = syn::parse_quote!(#ty::#variant_name);
                        let mut ds = Vec::new();
                        let mut es = Vec::new();
                        let mut bindings = Vec::new();
                        for (i, field) in variant.fields.iter().enumerate() {
                            let c = build(
                                rule,
                                &rule.shape(&named.module, Some(path), &field.ty),
                                stack,
                            )?;
                            let binding = quote::format_ident!("field_{i}");
                            let d = c.decode;
                            let e = c.encode;
                            ds.push(d);
                            es.push(quote!({ let value = #binding; #e }));
                            bindings.push(binding);
                        }
                        let (construct, pattern) = match &variant.fields {
                            syn::Fields::Named(f) => {
                                let names: Vec<_> =
                                    f.named.iter().map(|f| f.ident.as_ref().unwrap()).collect();
                                (
                                    quote!(#variant_path { #(#names: #ds,)* }),
                                    quote!(#variant_path { #(#names: #bindings,)* }),
                                )
                            }
                            syn::Fields::Unnamed(_) => (
                                quote!(#variant_path(#(#ds,)*)),
                                quote!(#variant_path(#(#bindings,)*)),
                            ),
                            syn::Fields::Unit => (quote!(#variant_path), quote!(#variant_path)),
                        };
                        let tag = index as u32;
                        decoders.push(quote!(#tag => #construct,));
                        encoders.push(quote!(#pattern => { out.put(&#tag); #(#es)* },));
                    }
                    Ok(ValueCodec {
                        decode: quote!(input.nested(|input| Ok(match input.take::<u32>()? { #(#decoders)* _ => return Err(mod_api::wire::WireError::Invalid("unknown enum variant")) }))?),
                        encode: quote!(match value { #(#encoders)* }),
                    })
                }
                _ => Err(format!("unsupported layout: {path}")),
            };
            stack.pop();
            result?
        }
        _ => return Err(format!("decoder not implemented: {shape:?}")),
    };
    Ok(pair)
}

fn fields(
    rule: &Rule,
    named: &crate::crossing::Named,
    path: &str,
    ty: &syn::Path,
    fields: &syn::Fields,
    stack: &mut Vec<String>,
) -> Result<ValueCodec, String> {
    if named.foreign
        && fields
            .iter()
            .any(|f| !matches!(f.vis, syn::Visibility::Public(_)))
    {
        return Err(format!("private foreign constructor: {path}"));
    }
    let mut ds = Vec::new();
    let mut es = Vec::new();
    for (i, f) in fields.iter().enumerate() {
        let c = build(rule, &rule.shape(&named.module, Some(path), &f.ty), stack)?;
        let member = f.ident.clone().map_or(
            syn::Member::Unnamed(syn::Index::from(i)),
            syn::Member::Named,
        );
        let d = c.decode;
        let e = c.encode;
        ds.push(quote!(#d));
        es.push(quote!({ let value = &value.#member; #e }));
    }
    let construct = match fields {
        syn::Fields::Named(f) => {
            let names = f.named.iter().map(|f| f.ident.as_ref().unwrap());
            quote!(#ty { #(#names: #ds,)* })
        }
        syn::Fields::Unnamed(_) => quote!(#ty(#(#ds,)*)),
        syn::Fields::Unit => quote!(#ty),
    };
    Ok(ValueCodec {
        decode: quote!(input.nested(|input| Ok(#construct))?),
        encode: quote!(#(#es)*),
    })
}
