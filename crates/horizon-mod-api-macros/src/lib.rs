//! Derive hook subscriptions from a trait implementation.
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ImplItem, ItemImpl, parse_macro_input};

include!(concat!(env!("OUT_DIR"), "/hooks.rs"));

fn distance(a: &str, b: &str) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, a) in a.bytes().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, b) in b.bytes().enumerate() {
            let old = row[j + 1];
            row[j + 1] = (row[j + 1] + 1)
                .min(row[j] + 1)
                .min(previous + usize::from(a != b));
            previous = old;
        }
    }
    row[b.len()]
}

/// Declare the hook methods implemented by this trait mod.
#[proc_macro_attribute]
pub fn hooks(_attribute: TokenStream, input: TokenStream) -> TokenStream {
    let mut implementation = parse_macro_input!(input as ItemImpl);
    let Some((_, trait_path, _)) = &implementation.trait_ else {
        return syn::Error::new_spanned(
            &implementation,
            "expected an impl of Hooks or RenderHooks",
        )
        .into_compile_error()
        .into();
    };
    let name = &trait_path.segments.last().unwrap().ident;
    let render = if name == "Hooks" {
        false
    } else if name == "RenderHooks" {
        true
    } else {
        return syn::Error::new_spanned(name, "expected Hooks or RenderHooks")
            .into_compile_error()
            .into();
    };
    let mut variants = Vec::new();
    for item in &implementation.items {
        let ImplItem::Fn(method) = item else { continue };
        let method_name = method.sig.ident.to_string();
        if !render && matches!(method_name.as_str(), "render" | "register") {
            continue;
        }
        match HOOKS
            .iter()
            .find(|(name, _, side)| *side == render && *name == method_name)
        {
            Some((_, variant, _)) => variants.push(format_ident!("{variant}")),
            None => {
                let closest = HOOKS
                    .iter()
                    .filter(|(_, _, side)| *side == render)
                    .min_by_key(|(name, _, _)| distance(&method_name, name))
                    .unwrap()
                    .0;
                return syn::Error::new_spanned(
                    &method.sig.ident,
                    format!("unknown hook `{method_name}`; closest hook is `{closest}`"),
                )
                .into_compile_error()
                .into();
            }
        }
    }
    let id = if render {
        format_ident!("RenderHookId")
    } else {
        format_ident!("HookId")
    };
    implementation.items.push(syn::parse_quote! {
        fn subscribed(&self) -> &'static [mod_api::#id] {
            &[#(mod_api::#id::#variants),*]
        }
    });
    quote!(#implementation).into()
}
