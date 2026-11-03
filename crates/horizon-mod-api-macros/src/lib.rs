//! Derive hook subscriptions from a trait implementation.
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    ImplItem, ImplItemFn, ItemImpl, Meta, Token, parse::Parser, parse_macro_input,
    punctuated::Punctuated,
};

/// Keep only attributes that can gate a method, including nested `cfg_attr`s.
fn cfg_meta(meta: &Meta) -> Option<Meta> {
    if meta.path().is_ident("cfg") {
        return Some(meta.clone());
    }
    let Meta::List(list) = meta else { return None };
    if !list.path.is_ident("cfg_attr") {
        return None;
    }
    let args = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()?;
    let mut args = args.iter();
    let condition = args.next()?;
    let gated: Vec<_> = args.filter_map(cfg_meta).collect();
    if gated.is_empty() {
        None
    } else {
        Some(syn::parse_quote!(cfg_attr(#condition, #(#gated),*)))
    }
}

fn cfg_attributes(method: &ImplItemFn) -> Vec<syn::Attribute> {
    method
        .attrs
        .iter()
        .filter_map(|attr| cfg_meta(&attr.meta))
        .map(|meta| syn::parse_quote!(#[#meta]))
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
    let (render, part) = match name.to_string().as_str() {
        "Hooks" => (false, None),
        "RenderHooks" => (true, None),
        "BlockPart" => (false, Some("block_subscribed")),
        "BlockEntityPart" => (false, Some("block_entity_subscribed")),
        "ItemPart" => (false, Some("item_subscribed")),
        "MenuPart" => (false, Some("menu_subscribed")),
        _ => {
            return syn::Error::new_spanned(name, "expected a hook or part trait")
                .into_compile_error()
                .into();
        }
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
    let subscribed_name = format_ident!("{}", part.unwrap_or("subscribed"));
    let subscribed = syn::parse_quote! {
        fn #subscribed_name(&self) -> &'static [Option<mod_api::#id>] {
            &[#(#(#cfgs)* mod_api::#ids::#methods),*]
        }
    };
    implementation.items.push(subscribed);
    let export = if export && !render && part.is_none() {
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

/// Mark a part implementation and derive the scoped hooks it overrides.
#[proc_macro_attribute]
pub fn horizon_part(_attribute: TokenStream, input: TokenStream) -> TokenStream {
    expand(parse_macro_input!(input as ItemImpl), false)
}

/// Derive subscriptions for internal test fixtures that construct their own mods.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn __subscriptions(_attribute: TokenStream, input: TokenStream) -> TokenStream {
    expand(parse_macro_input!(input as ItemImpl), false)
}
