//! The shape of an injection point (#589): what a `#[point]` function hands its listeners. Shared
//! by the `#[point]` attribute (the check at the function's head) and the mod API generator (the
//! slot statics and `points.json`, which `#[path]`s this file in), so the two always agree.
// Each of the two uses its own part of the shape.
#![allow(dead_code)]
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FnArg, Pat, Type};

/// Where a point's function runs, as a hook's `side`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Server,
    Client,
    Both,
}

impl Side {
    pub fn name(self) -> &'static str {
        match self {
            Self::Server => "server",
            Self::Client => "client",
            Self::Both => "both",
        }
    }
}

/// One argument a point passes: the parameter's binding and type.
pub struct Arg {
    pub name: syn::Ident,
    pub ty: Type,
}

/// A `#[point]` function's point.
pub struct Shape {
    /// Unique among the game's points: the function's name, or the attribute's `name`.
    pub name: String,
    pub side: Side,
    /// The function takes `&mut self`: its listeners get the side's writing host.
    pub mutable: bool,
    /// Its listeners run per block, per entity or per frame: the docs and build log say so.
    pub hot: bool,
    /// The parameters with API types, in order. The others aren't passed: an internal type
    /// would become API.
    pub args: Vec<Arg>,
}

/// The point of the function `sig`, from `#[point(side = "…", name = "…", hot)]`'s arguments.
pub fn shape(attribute: TokenStream, sig: &syn::Signature) -> syn::Result<Shape> {
    let (mut name, mut side, mut hot) = (sig.ident.to_string(), None, false);
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("hot") {
            hot = true;
        } else if meta.path.is_ident("name") {
            name = meta.value()?.parse::<syn::LitStr>()?.value();
        } else if meta.path.is_ident("side") {
            let value = meta.value()?.parse::<syn::LitStr>()?;
            side = Some(match value.value().as_str() {
                "server" => Side::Server,
                "client" => Side::Client,
                "both" => Side::Both,
                _ => return Err(syn::Error::new(value.span(), "side is server, client or both")),
            });
        } else {
            return Err(meta.error("a point takes side = \"…\", name = \"…\" and hot"));
        }
        Ok(())
    });
    syn::parse::Parser::parse2(parser, attribute)?;
    let side = side.ok_or_else(|| syn::Error::new(sig.ident.span(), "a point needs side = \"server\", \"client\" or \"both\""))?;
    let mutable = match sig.inputs.first() {
        Some(FnArg::Receiver(receiver)) if receiver.reference.is_some() => receiver.mutability.is_some(),
        _ => {
            return Err(syn::Error::new(sig.ident.span(), "a point is a `&self` or `&mut self` method of `Game`"));
        }
    };
    let args = sig
        .inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(typed) => match &*typed.pat {
                Pat::Ident(pat) if pat.by_ref.is_none() && pat.subpat.is_none() && api_typed(&typed.ty) => {
                    Some(Arg { name: pat.ident.clone(), ty: (*typed.ty).clone() })
                }
                _ => None,
            },
            FnArg::Receiver(_) => None,
        })
        .collect();
    Ok(Shape { name, side, mutable, hot, args })
}

/// Whether the API can carry `ty` to a listener: a scalar, or `&str`.
fn api_typed(ty: &Type) -> bool {
    const SCALARS: [&str; 15] = [
        "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "usize",
    ];
    match ty {
        Type::Path(path) => path.qself.is_none() && SCALARS.iter().any(|scalar| path.path.is_ident(scalar)),
        Type::Reference(reference) => {
            reference.mutability.is_none() && matches!(&*reference.elem, Type::Path(path) if path.path.is_ident("str"))
        }
        _ => false,
    }
}

impl Shape {
    /// The slot's static in the game's generated `points` module.
    pub fn slot(&self) -> syn::Ident {
        syn::Ident::new(&self.name.to_uppercase(), Span::call_site())
    }

    /// The arguments' record, the listener's second parameter: `(T, …,)`.
    pub fn record(&self) -> TokenStream {
        let types = self.args.iter().map(|arg| &arg.ty);
        quote!((#(#types,)*))
    }

    /// The listener's function type.
    pub fn listener(&self) -> TokenStream {
        let record = self.record();
        let host = match (self.mutable, self.side) {
            (false, _) => quote!(&dyn ::mod_api::Host),
            (true, Side::Server) => quote!(&mut dyn ::mod_api::ServerHost),
            (true, Side::Client) => quote!(&mut dyn ::mod_api::ClientHost),
            (true, Side::Both) => quote!(::mod_api::HostSide<'_>),
        };
        quote!(fn(#host, &mut #record))
    }
}
