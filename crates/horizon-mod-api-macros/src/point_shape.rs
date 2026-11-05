//! The shape of an injection point (#589): what a `#[point]` function hands its listeners. Shared
//! by the `#[point]` attribute (the check at the function's head) and the mod API generator (the
//! slot statics and `points.json`, which `#[path]`s this file in), so the two always agree.
// Each of the two uses its own part of the shape.
#![allow(dead_code)]
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
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

/// What a point's listeners may do with the function's return value.
pub enum Returns {
    /// `()`: a head listener may `cancel`, and the function returns at once.
    Unit,
    /// `Result<()>`: the same, returning `Ok(())`.
    UnitResult,
    /// An API-typed value: a head listener may set `result`, which the function returns at once;
    /// a return listener sees the value in `result` and may change it.
    Value(Type),
    /// A value the API can't carry: the listeners neither stop the function nor see its value.
    Opaque,
}

/// How a point's function takes `self`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Receiver {
    /// A free function, or an associated one without `self`.
    None,
    /// `&self`.
    Shared,
    /// `&mut self`.
    Mut,
}

/// A `#[point]` function's point.
pub struct Shape {
    /// Unique among the game's points: the attribute's `name`, or the function's own.
    pub name: String,
    /// The side its `Game` method runs on, which picks its API listeners' host. A point off `Game`
    /// has none, and its API listeners get no host.
    pub side: Option<Side>,
    pub receiver: Receiver,
    /// The parameters with API types, in order. The others aren't passed: an internal type
    /// would become API.
    pub args: Vec<Arg>,
    pub returns: Returns,
    /// The value it returns borrows (`-> &T`, `-> Option<&mut T>`): the receiver is still
    /// borrowed when the body ends, so its return place is raw only, and gets the value itself.
    pub borrowed_return: bool,
    /// It has a raw form (`raw` in the attribute): the staging pass declared its raw record and
    /// listener types beside it ([`Shape::raw_items`]).
    pub raw: bool,
    /// A free function (`free` in the attribute), not an associated one.
    pub free: bool,
}

/// Whether the staging pass gives a function with these `attrs` and `sig` a point: one predicate for
/// the pass and for the generator that lists the points, so the two always agree. Left out:
/// the hooks' own functions (`native_*`, whose bodies the generator reads), anything compiled only
/// under a `cfg`, `const`, `async` and `unsafe` functions, methods taking `self` by value,
/// `impl Trait` in the signature, `-> !`, `#[inline(always)]`, and parameters named `cancel` or
/// `result`, which the record has its own of. A `Game` method also needs a `&self` or `&mut self`
/// receiver: its listeners get the game as their host.
pub fn eligible(attrs: &[syn::Attribute], sig: &syn::Signature) -> bool {
    let impl_trait = |ty: &Type| matches!(ty, Type::ImplTrait(_));
    !sig.ident.to_string().starts_with("native_")
        && sig.constness.is_none()
        && sig.asyncness.is_none()
        && sig.unsafety.is_none()
        && sig.abi.is_none()
        && !matches!(sig.inputs.first(), Some(FnArg::Receiver(receiver)) if receiver.reference.is_none())
        && !matches!(&sig.output, syn::ReturnType::Type(_, ty) if impl_trait(ty) || matches!(**ty, Type::Never(_)))
        && sig.inputs.iter().all(|input| match input {
            FnArg::Typed(typed) => {
                !impl_trait(&typed.ty)
                    && !matches!(&*typed.pat, Pat::Ident(pat) if pat.ident == "cancel" || pat.ident == "result")
            }
            FnArg::Receiver(_) => true,
        })
        && !attrs.iter().any(|attr| {
            let path = attr.path();
            path.is_ident("cfg")
                || path.is_ident("cfg_attr")
                || path.is_ident("test")
                || (path.is_ident("inline")
                    && attr
                        .parse_args::<syn::Ident>()
                        .is_ok_and(|arg| arg == "always"))
        })
}

/// Whether `ty` holds a reference or a named lifetime.
fn borrows(ty: &Type) -> bool {
    struct Find(bool);
    impl<'ast> syn::visit::Visit<'ast> for Find {
        fn visit_type_reference(&mut self, _: &'ast syn::TypeReference) {
            self.0 = true;
        }
        fn visit_lifetime(&mut self, _: &'ast syn::Lifetime) {
            self.0 = true;
        }
    }
    let mut find = Find(false);
    syn::visit::Visit::visit_type(&mut find, ty);
    find.0
}

/// Whether a point with this `sig`, in an `impl` with `impl_generics` (none for a free function),
/// has a raw form: its listener's type is a plain function pointer, so neither the function nor its
/// `impl` may have type or const parameters, nor its signature an `impl Trait` anywhere.
pub fn raw_able(sig: &syn::Signature, impl_generics: Option<&syn::Generics>) -> bool {
    struct Find(bool);
    impl<'ast> syn::visit::Visit<'ast> for Find {
        fn visit_type_impl_trait(&mut self, _: &'ast syn::TypeImplTrait) {
            self.0 = true;
        }
    }
    let mut find = Find(false);
    syn::visit::Visit::visit_signature(&mut find, sig);
    if find.0 {
        return false;
    }
    let plain = |generics: &syn::Generics| {
        generics
            .params
            .iter()
            .all(|param| matches!(param, syn::GenericParam::Lifetime(_)))
    };
    plain(&sig.generics) && impl_generics.is_none_or(plain)
}

/// The point of the function `sig`, from `#[point(side = "…", name = "…", raw, free)]`'s arguments.
pub fn shape(attribute: TokenStream, sig: &syn::Signature) -> syn::Result<Shape> {
    let (mut name, mut side, mut raw, mut free) = (sig.ident.to_string(), None, false, false);
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("raw") {
            raw = true;
        } else if meta.path.is_ident("free") {
            free = true;
        } else if meta.path.is_ident("name") {
            name = meta.value()?.parse::<syn::LitStr>()?.value();
        } else if meta.path.is_ident("side") {
            let value = meta.value()?.parse::<syn::LitStr>()?;
            side = Some(match value.value().as_str() {
                "server" => Side::Server,
                "client" => Side::Client,
                "both" => Side::Both,
                _ => {
                    return Err(syn::Error::new(
                        value.span(),
                        "side is server, client or both",
                    ));
                }
            });
        } else {
            return Err(meta.error("a point takes side = \"…\", name = \"…\", raw and free"));
        }
        Ok(())
    });
    syn::parse::Parser::parse2(parser, attribute)?;
    let receiver = match sig.inputs.first() {
        Some(FnArg::Receiver(receiver)) if receiver.reference.is_none() => {
            return Err(syn::Error::new_spanned(
                receiver,
                "a point's function takes `self` by reference, or not at all",
            ));
        }
        Some(FnArg::Receiver(receiver)) if receiver.mutability.is_some() => Receiver::Mut,
        Some(FnArg::Receiver(_)) => Receiver::Shared,
        _ => Receiver::None,
    };
    if side.is_some() && receiver == Receiver::None {
        return Err(syn::Error::new(
            sig.ident.span(),
            "a point with a side is a `&self` or `&mut self` method of `Game`",
        ));
    }
    let args = sig
        .inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(typed) => match &*typed.pat {
                Pat::Ident(pat)
                    if pat.by_ref.is_none() && pat.subpat.is_none() && api_typed(&typed.ty) =>
                {
                    Some(Arg {
                        name: pat.ident.clone(),
                        ty: (*typed.ty).clone(),
                    })
                }
                _ => None,
            },
            FnArg::Receiver(_) => None,
        })
        .collect::<Vec<_>>();
    if let Some(arg) = args
        .iter()
        .find(|arg| arg.name == "cancel" || arg.name == "result")
    {
        return Err(syn::Error::new(
            arg.name.span(),
            "a point's record has its own `cancel` and `result`: rename the parameter",
        ));
    }
    let borrowed_return = matches!(&sig.output, syn::ReturnType::Type(_, ty) if borrows(ty));
    // Without a receiver, a borrowed value may borrow a text argument: the function gets that text
    // as passed, so the record doesn't carry it.
    let mut args = args;
    if borrowed_return && receiver == Receiver::None {
        args.retain(|arg| !matches!(arg.ty, Type::Reference(_)));
    }
    let returns = match &sig.output {
        syn::ReturnType::Default => Returns::Unit,
        syn::ReturnType::Type(_, ty) if unit_result(ty) => Returns::UnitResult,
        syn::ReturnType::Type(_, ty) if scalar(ty) => Returns::Value((**ty).clone()),
        syn::ReturnType::Type(..) => Returns::Opaque,
    };
    Ok(Shape {
        name,
        side,
        receiver,
        args,
        returns,
        borrowed_return,
        raw,
        free,
    })
}

/// `Result<()>` (the game's `anyhow` alias) or `Result<(), E>`.
fn unit_result(ty: &Type) -> bool {
    let Type::Path(path) = ty else { return false };
    let Some(last) = path.path.segments.last() else {
        return false;
    };
    let syn::PathArguments::AngleBracketed(generics) = &last.arguments else {
        return false;
    };
    last.ident == "Result"
        && matches!(generics.args.first(), Some(syn::GenericArgument::Type(Type::Tuple(unit))) if unit.elems.is_empty())
}

/// A scalar the API carries by value.
fn scalar(ty: &Type) -> bool {
    matches!(ty, Type::Path(_)) && api_typed(ty)
}

/// `tick_return` as `TickReturn`.
fn camel(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// Whether the API can carry `ty` to a listener: a scalar, or `&str` with its lifetime elided (a listener
/// may replace that text with its own, which a named lifetime such as `'static` couldn't hold).
fn api_typed(ty: &Type) -> bool {
    const SCALARS: [&str; 15] = [
        "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16",
        "u32", "u64", "usize",
    ];
    match ty {
        Type::Path(path) => {
            path.qself.is_none() && SCALARS.iter().any(|scalar| path.path.is_ident(scalar))
        }
        Type::Reference(reference) => {
            reference.mutability.is_none()
                && reference.lifetime.is_none()
                && matches!(&*reference.elem, Type::Path(path) if path.path.is_ident("str"))
        }
        _ => false,
    }
}

impl Shape {
    /// The `PointId` variant of its head: `Tick` for `tick`.
    pub fn variant(&self) -> syn::Ident {
        syn::Ident::new(&camel(&self.name), Span::call_site())
    }

    /// The `PointId` variant of its return.
    pub fn return_variant(&self) -> syn::Ident {
        syn::Ident::new(&camel(&format!("{}_return", self.name)), Span::call_site())
    }

    /// The record its listeners get, in `mod_points`: `TickArgs` for `tick`.
    pub fn record(&self) -> syn::Ident {
        syn::Ident::new(&format!("{}Args", camel(&self.name)), Span::call_site())
    }

    /// Whether the record borrows (a `&str` parameter).
    fn borrows(&self) -> bool {
        self.args
            .iter()
            .any(|arg| matches!(arg.ty, Type::Reference(_)))
    }

    /// The record's declaration in `mod_points`, documented from the function's `target`.
    pub fn record_item(&self, target: &str) -> TokenStream {
        let record = self.record();
        let lifetime = if self.borrows() {
            quote!(<'a>)
        } else {
            quote!()
        };
        let names: Vec<_> = self.args.iter().map(|arg| &arg.name).collect();
        // Text crosses as a `Cow`, so a listener may replace it with an owned `String`.
        let (types, fields): (Vec<_>, Vec<_>) = self
            .args
            .iter()
            .map(|arg| match &arg.ty {
                Type::Reference(_) => (quote!(&'a str), quote!(::std::borrow::Cow<'a, str>)),
                ty => (quote!(#ty), quote!(#ty)),
            })
            .unzip();
        let inits = self.args.iter().map(|arg| {
            let name = &arg.name;
            match &arg.ty {
                Type::Reference(_) => quote!(#name: ::std::borrow::Cow::Borrowed(#name)),
                _ => quote!(#name),
            }
        });
        let field_docs = names.iter().map(|name| {
            format!(
                "`{name}`: a head listener may change it, and the function runs with the change."
            )
        });
        let (extra, extra_init) = match &self.returns {
            Returns::Unit | Returns::UnitResult => (
                quote! {
                    /// Set by a head listener: the function returns at once, skipping its body.
                    pub cancel: bool,
                },
                quote!(cancel: false,),
            ),
            Returns::Value(ty) => (
                quote! {
                    /// Set by a head listener: the function returns this at once, skipping its body.
                    /// Return listeners find the value it returns here, and may change it.
                    pub result: Option<#ty>,
                },
                quote!(result: None,),
            ),
            Returns::Opaque => (quote!(), quote!()),
        };
        let doc = format!(
            "The `{}` point's record: what its listeners get from `{target}`, a slot filled at load.",
            self.name
        );
        quote! {
            #[doc = #doc]
            #[non_exhaustive]
            pub struct #record #lifetime {
                #( #[doc = #field_docs] pub #names: #fields, )*
                #extra
            }
            impl #lifetime #record #lifetime {
                /// The record of a call, before its listeners run (the game's side of the point).
                #[doc(hidden)]
                #[inline]
                pub fn __new(#(#names: #types),*) -> Self {
                    Self { #(#inits,)* #extra_init }
                }
            }
        }
    }

    /// The `Points` methods for this point (its head, then its return when it has one), their
    /// `PointId` variants, and the `Option<PointId>` consts `#[horizon_mod]` subscribes a mod by:
    /// generated into `mod_points`.
    pub fn api_items(&self, target: &str) -> (TokenStream, TokenStream, TokenStream) {
        let record = self.record();
        let record = if self.borrows() {
            quote!(#record<'_>)
        } else {
            quote!(#record)
        };
        let host = match (self.receiver, self.side) {
            (_, None) => quote!(),
            (Receiver::Mut, Some(Side::Server)) => quote!(host: &mut dyn ::mod_api::ServerHost,),
            (Receiver::Mut, Some(Side::Client)) => quote!(host: &mut dyn ::mod_api::ClientHost,),
            (Receiver::Mut, Some(Side::Both)) => quote!(host: ::mod_api::HostSide<'_>,),
            (_, Some(_)) => quote!(host: &dyn ::mod_api::Host,),
        };
        let unused = if self.side.is_some() {
            quote!(let _ = (host, args);)
        } else {
            quote!(let _ = args;)
        };
        let mut places = vec![(
            self.name.clone(),
            "head: before its body runs, with the arguments it was called with",
        )];
        if self.has_return() {
            places.push((
                format!("{}_return", self.name),
                "return: after its body, with the value it returns",
            ));
        }
        let (mut methods, mut variants, mut consts) = (quote!(), quote!(), quote!());
        for (method, place) in places {
            let method = syn::Ident::new(&method, Span::call_site());
            let variant = syn::Ident::new(&camel(&method.to_string()), Span::call_site());
            let doc = format!("`{target}`'s {place}.");
            methods.extend(quote! {
                #[doc = #doc]
                fn #method(&self, #host args: &mut #record) {
                    #unused
                }
            });
            variants.extend(quote!(#variant,));
            consts.extend(quote! {
                #[doc(hidden)]
                #[allow(non_upper_case_globals)]
                pub const #method: Option<PointId> = Some(PointId::#variant);
            });
        }
        if self.raw && !self.has_return() {
            let variant = self.return_variant();
            variants.extend(quote!(#variant,));
        }
        (methods, variants, consts)
    }

    /// Whether its API listeners also run after the body: not when the value borrows.
    pub fn has_return(&self) -> bool {
        !self.borrowed_return
    }

    /// Whether it has a return place: for API listeners, or for raw ones only.
    pub fn at_return(&self) -> bool {
        self.has_return() || self.raw
    }

    /// The raw record's type, declared beside the function: `__TickRaw` for `tick`.
    pub fn raw_record(&self) -> syn::Ident {
        format_ident!("__{}Raw", camel(&self.name))
    }

    /// The raw head listeners' function type: `__TickListener`.
    pub fn raw_listener(&self) -> syn::Ident {
        format_ident!("__{}Listener", camel(&self.name))
    }

    /// The raw return listeners' function type: `__TickReturnListener`.
    pub fn raw_return_listener(&self) -> syn::Ident {
        format_ident!("__{}ReturnListener", camel(&self.name))
    }

    /// What the raw record holds for the function's outcome: `cancel`, a `result` the head may set
    /// to return at once, or nothing (a borrowed value, which no head listener can make).
    pub fn raw_outcome(&self, sig: &syn::Signature) -> RawOutcome {
        match (&self.returns, &sig.output) {
            (Returns::Unit | Returns::UnitResult, _) => RawOutcome::Cancel,
            _ if self.borrowed_return => RawOutcome::None,
            (_, syn::ReturnType::Type(_, ty)) => RawOutcome::Result((**ty).clone()),
            (_, syn::ReturnType::Default) => RawOutcome::Cancel,
        }
    }

    /// The raw record and listener types of this point, declared by the staging pass beside the
    /// function, in its module, so the parameters' types name what they name there. `self_ty` is
    /// the `impl`'s type (none for a free function), standing in for `Self`; `impl_generics` its
    /// lifetimes, bound by the listener type like the function's own.
    ///
    /// The record holds a `&mut` to each parameter (generic over their types, so its declaration
    /// needs none of them) and to the outcome; the listener type names the real types. Text the API
    /// carries is the API record's `Cow`, the same place an API listener changes.
    pub fn raw_items(
        &self,
        sig: &syn::Signature,
        self_ty: Option<&Type>,
        impl_generics: Option<&syn::Generics>,
    ) -> TokenStream {
        let mut lifetimes: Vec<syn::Lifetime> = impl_generics
            .into_iter()
            .flat_map(|generics| generics.lifetimes())
            .chain(sig.generics.lifetimes())
            .map(|param| param.lifetime.clone())
            .collect();
        let mut seen = std::collections::BTreeSet::new();
        lifetimes.retain(|lifetime| seen.insert(lifetime.ident.to_string()));
        let concrete = |ty: &Type| {
            let mut ty = ty.clone();
            if let Some(self_ty) = self_ty {
                struct Replace<'a>(&'a Type);
                impl syn::visit_mut::VisitMut for Replace<'_> {
                    fn visit_type_mut(&mut self, ty: &mut Type) {
                        if matches!(ty, Type::Path(path) if path.qself.is_none() && path.path.is_ident("Self"))
                        {
                            *ty = self.0.clone();
                        } else {
                            syn::visit_mut::visit_type_mut(self, ty);
                        }
                    }
                }
                syn::visit_mut::VisitMut::visit_type_mut(&mut Replace(self_ty), &mut ty);
            }
            ty
        };
        let fields = raw_fields(sig);
        let names: Vec<_> = fields.iter().map(|field| &field.name).collect();
        let params: Vec<_> = (0..fields.len()).map(|i| format_ident!("T{i}")).collect();
        let args: Vec<_> = fields
            .iter()
            .map(|field| {
                if self.args.iter().any(|arg| arg.name == field.name)
                    && matches!(field.ty, Type::Reference(_))
                {
                    quote!(::std::borrow::Cow<'_, str>)
                } else {
                    let ty = concrete(&field.ty);
                    quote!(#ty)
                }
            })
            .collect();
        let (outcome, outcome_param, outcome_arg) = match self.raw_outcome(sig) {
            RawOutcome::Cancel => (
                quote! {
                    /// Set by a head listener: the function returns at once, skipping its body.
                    pub cancel: &'r mut bool,
                },
                quote!(),
                quote!(),
            ),
            RawOutcome::Result(ty) => {
                let ty = concrete(&ty);
                (
                    quote! {
                        /// Set by a head listener: the function returns it at once, skipping its
                        /// body. A return listener finds the value returned here, and may change it
                        /// but not take it.
                        pub result: &'r mut Option<R>,
                    },
                    quote!(R,),
                    quote!(#ty,),
                )
            }
            RawOutcome::None => (quote!(), quote!(), quote!()),
        };
        let (record, listener) = (self.raw_record(), self.raw_listener());
        let empty = fields.is_empty() && matches!(self.raw_outcome(sig), RawOutcome::None);
        let (generics, record_ty) = if empty {
            (quote!(), quote!(#record))
        } else {
            (
                quote!(<'r, #(#params,)* #outcome_param>),
                quote!(#record<'r, #(#args,)* #outcome_arg>),
            )
        };
        let receiver = match (self.receiver, self_ty) {
            (Receiver::Mut, Some(ty)) => quote!(&'r mut #ty,),
            (Receiver::Shared, Some(ty)) => quote!(&'r #ty,),
            _ => quote!(),
        };
        let doc = format!(
            "The `{}` point's raw record: a `&mut` to each argument, internal types included.",
            self.name
        );
        let listener_doc = format!(
            "The `{}` point's raw listener: a mod's function of this type, registered by name.",
            self.name
        );
        let mut items = quote! {
            #[doc = #doc]
            #[doc(hidden)]
            pub struct #record #generics {
                #( pub #names: &'r mut #params, )*
                #outcome
            }
            #[doc = #listener_doc]
            #[doc(hidden)]
            pub type #listener = for<'r, #(#lifetimes),*> fn(#receiver &'r mut #record_ty);
        };
        // At the return the arguments are spent: a return listener gets the receiver (unless the
        // value borrows it) and the value returned, which it may change.
        let receiver = if self.borrowed_return {
            quote!()
        } else {
            receiver
        };
        let value = match &sig.output {
            syn::ReturnType::Type(_, ty) => {
                let ty = concrete(ty);
                quote!(&'r mut #ty)
            }
            syn::ReturnType::Default => quote!(),
        };
        let return_listener = self.raw_return_listener();
        items.extend(quote! {
            /// The point's raw return listener: it gets the value the function returns.
            #[doc(hidden)]
            pub type #return_listener = for<'r, #(#lifetimes),*> fn(#receiver #value);
        });
        items
    }
}

/// What a raw record holds for its function's outcome ([`Shape::raw_outcome`]).
pub enum RawOutcome {
    Cancel,
    Result(Type),
    None,
}

/// One field of a raw record: a parameter, by its binding, or `arg<i>` for a pattern.
pub struct RawField {
    pub name: syn::Ident,
    /// The parameter's position among the function's inputs, receiver included.
    pub input: usize,
    pub ty: Type,
}

/// The raw record's fields: every typed parameter of `sig`, in order.
pub fn raw_fields(sig: &syn::Signature) -> Vec<RawField> {
    sig.inputs
        .iter()
        .enumerate()
        .filter_map(|(input, arg)| match arg {
            FnArg::Typed(typed) => Some(RawField {
                name: match &*typed.pat {
                    Pat::Ident(pat) if pat.by_ref.is_none() && pat.subpat.is_none() => {
                        pat.ident.clone()
                    }
                    _ => format_ident!("arg{input}"),
                },
                input,
                ty: (*typed.ty).clone(),
            }),
            FnArg::Receiver(_) => None,
        })
        .collect()
}
