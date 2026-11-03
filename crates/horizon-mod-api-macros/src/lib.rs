//! Derive hook subscriptions from a trait implementation.
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ImplItem, ItemImpl, parse_macro_input};

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
    let methods: Vec<_> = implementation
        .items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Fn(method)
                if render
                    || !matches!(method.sig.ident.to_string().as_str(), "render" | "register") =>
            {
                Some(&method.sig.ident)
            }
            _ => None,
        })
        .collect();
    let id = if render {
        format_ident!("RenderHookId")
    } else {
        format_ident!("HookId")
    };
    let ids = if render {
        format_ident!("render_hook_ids")
    } else {
        format_ident!("hook_ids")
    };
    let subscribed = syn::parse_quote! {
        fn subscribed(&self) -> &'static [mod_api::#id] {
            &[#(mod_api::#ids::#methods),*]
        }
    };
    implementation.items.push(subscribed);
    quote!(#implementation).into()
}
