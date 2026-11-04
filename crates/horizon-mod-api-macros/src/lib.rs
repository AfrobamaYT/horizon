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

fn uninspectable(item: &ImplItem) -> bool {
    matches!(item, ImplItem::Macro(_) | ImplItem::Verbatim(_))
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
    if part.is_some()
        && let Some(item) = implementation.items.iter().find(|item| uninspectable(item))
    {
        let ty = &implementation.self_ty;
        return syn::Error::new_spanned(
            item,
            format!("uninspectable items in impl {trait_path} for {ty} are not supported by horizon_part; write hook methods explicitly", trait_path = quote!(#trait_path), ty = quote!(#ty)),
        )
        .into_compile_error()
        .into();
    }
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
        quote! {
            pub fn hooks() -> Box<dyn mod_api::Hooks> { Box::new(<#ty as Default>::default()) }
            /// The mod's hooks for the guest glue in `mod_api::wasm`, when built to `mod.wasm`.
            #[cfg(target_arch = "wasm32")]
            #[unsafe(no_mangle)]
            fn __horizon_mod_hooks() -> Box<dyn mod_api::Hooks> { hooks() }
        }
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
///
/// Hook methods must be written explicitly, not generated inside the impl:
///
/// ```compile_fail
/// use horizon_mod_api_macros::horizon_part;
/// trait BlockPart { fn tick(&self) {} }
/// struct Block;
/// macro_rules! hook { () => { fn tick(&self) {} }; }
/// #[horizon_part]
/// impl BlockPart for Block { hook!(); }
/// ```
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

/// `#[wasm(via = Form)]` on a `mod_api` type, or on the `pub use` that exports another crate's:
/// the type crosses to wasm mods as `Form`, through `From<&Type> for Form` and
/// `TryFrom<Form> for Type`, which live with the type. The bindings generator reads the
/// attribute; this checks the two conversions exist.
#[proc_macro_attribute]
pub fn wasm(attribute: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as syn::Item);
    let via = match syn::parse::<WasmVia>(attribute) {
        Ok(via) => via.form,
        Err(e) => return e.to_compile_error().into(),
    };
    let ty = match &item {
        syn::Item::Struct(item) => item.ident.clone(),
        syn::Item::Enum(item) => item.ident.clone(),
        syn::Item::Use(item) => match last_use_name(&item.tree) {
            Some(name) => name,
            None => {
                return syn::Error::new_spanned(item, "#[wasm] exports one type")
                    .to_compile_error()
                    .into();
            }
        },
        item => {
            return syn::Error::new_spanned(item, "#[wasm] goes on a type or its `pub use`")
                .to_compile_error()
                .into();
        }
    };
    quote! {
        #item
        const _: () = {
            fn crosses_as_its_form()
            where
                #via: for<'a> ::core::convert::From<&'a #ty>,
                #ty: ::core::convert::TryFrom<#via>,
            {
            }
        };
    }
    .into()
}

/// `via = Form`.
struct WasmVia {
    form: syn::Type,
}

impl syn::parse::Parse for WasmVia {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        if key != "via" {
            return Err(syn::Error::new(key.span(), "expected `via = Form`"));
        }
        input.parse::<syn::Token![=]>()?;
        Ok(Self {
            form: input.parse()?,
        })
    }
}

/// The name a `use` tree of one path brings in.
fn last_use_name(tree: &syn::UseTree) -> Option<syn::Ident> {
    match tree {
        syn::UseTree::Path(path) => last_use_name(&path.tree),
        syn::UseTree::Name(name) => Some(name.ident.clone()),
        syn::UseTree::Rename(rename) => Some(rename.rename.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbatim_items_are_uninspectable() {
        let item = ImplItem::Verbatim(quote!(future syntax));
        assert!(uninspectable(&item));
    }
}
