//! Derive hook subscriptions from a trait implementation.
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ImplItem, ImplItemFn, ItemImpl, parse_macro_input};

/// A method's `#[cfg]` attributes: they gate the method body, so the same
/// conditions gate its derived subscription entry. `cfg_attr` is left out —
/// it expands to other attributes, not a condition of its own.
fn cfg_attributes(method: &ImplItemFn) -> Vec<&syn::Attribute> {
    method
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .collect()
}

fn expand(mut implementation: ItemImpl, export: bool) -> TokenStream {
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
    let mut methods = Vec::new();
    let mut cfgs = Vec::new();
    for item in &implementation.items {
        if let ImplItem::Fn(method) = item {
            methods.push(&method.sig.ident);
            cfgs.push(cfg_attributes(method));
        }
    }
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
        fn subscribed(&self) -> &'static [Option<mod_api::#id>] {
            &[#(#(#cfgs)* mod_api::#ids::#methods),*]
        }
    };
    implementation.items.push(subscribed);
    let export = if export && !render {
        let ty = &implementation.self_ty;
        quote!(pub fn hooks() -> Box<dyn mod_api::Hooks> { Box::new(<#ty as Default>::default()) })
    } else {
        quote!()
    };
    quote!(#implementation #export).into()
}

/// Mark a trait mod or its renderer and derive its hook subscriptions.
#[proc_macro_attribute]
pub fn horizon_mod(_attribute: TokenStream, input: TokenStream) -> TokenStream {
    expand(parse_macro_input!(input as ItemImpl), true)
}

/// Derive subscriptions for internal test fixtures that construct their own mods.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn __subscriptions(_attribute: TokenStream, input: TokenStream) -> TokenStream {
    expand(parse_macro_input!(input as ItemImpl), false)
}
