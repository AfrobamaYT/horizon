//! The staged game's internals table (#589 W): for every type and function `internals.json` lists
//! that can cross to a wasm mod, the code the wasm runtime reaches them through
//! (`mod_points::internals::Table`). Generated into the staged copy only, as the crate root's
//! `__internals`; the working tree has none of it.
//!
//! - **Types.** Each walked type gets one node function: a step into a field, an element or an
//!   entry, a copy-typed field's read and write, its case, its length.
//! - **Copies.** Each listed copy type gets one encoder, and a decoder when it can be built (no
//!   borrowed or shared part); a field or argument of any other copy shape is converted inline.
//! - **Functions.** Each listed function whose receiver, arguments and return can all cross gets a
//!   thunk: copies are decoded, references come in as nodes, and a returned reference goes back
//!   as a node.
use crate::crossing::{Collection, Crossing, Rule, Scalar, Shape, Wrapper, bare};
use crate::internals::Internal;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::{BTreeMap, HashMap};

/// The game's crate.
const GAME: &str = "horizon_client";

/// What the generator made, and what it left out.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub types: usize,
    pub thunks: usize,
    /// Functions with a part that can't cross, or a form a thunk can't make (a generic, a `self`
    /// taken by value, a returned value that is no copy).
    pub skipped: usize,
    /// Layouts whose accessibility or field set cannot be proven from the manifest.
    pub skipped_types: BTreeMap<&'static str, usize>,
    /// Functions refused at each stage of thunk generation.
    pub skipped_functions: BTreeMap<&'static str, usize>,
}

struct Gen<'r> {
    rule: &'r Rule,
    /// Each walked type's id, by its spelling.
    ids: HashMap<String, u32>,
    /// Each walked type, in id order: its spelling and node function.
    nodes: Vec<(String, TokenStream)>,
    /// Each listed copy type's codec, by path: already generated.
    codecs: HashMap<String, bool>,
    items: Vec<TokenStream>,
    skip_reason: &'static str,
}

/// A type's Rust spelling, from the game crate's root.
fn spell(shape: &Shape) -> Option<TokenStream> {
    Some(match shape {
        Shape::Scalar(scalar) => match scalar {
            Scalar::Bool => quote!(bool),
            Scalar::Char => quote!(char),
            Scalar::Signed(name) | Scalar::Unsigned(name) | Scalar::Float(name) => {
                let name = format_ident!("{name}");
                quote!(#name)
            }
            Scalar::Str => quote!(str),
            Scalar::String => quote!(::std::string::String),
        },
        Shape::Unit => quote!(()),
        Shape::Ref { mutable, to } => {
            let to = spell(to)?;
            if *mutable {
                quote!(&'static mut #to)
            } else {
                quote!(&'static #to)
            }
        }
        Shape::Tuple(parts) => {
            let parts = parts.iter().map(spell).collect::<Option<Vec<_>>>()?;
            quote!((#(#parts,)*))
        }
        Shape::Array(elem, n) => {
            let elem = spell(elem)?;
            quote!([#elem; #n])
        }
        Shape::Slice(elem) => {
            let elem = spell(elem)?;
            quote!([#elem])
        }
        Shape::Option(inner) => {
            let inner = spell(inner)?;
            quote!(::std::option::Option<#inner>)
        }
        Shape::Wrapped(wrapper, inner) => {
            let inner = spell(inner)?;
            match wrapper {
                Wrapper::Box => quote!(::std::boxed::Box<#inner>),
                Wrapper::Arc => quote!(::std::sync::Arc<#inner>),
                Wrapper::Rc => quote!(::std::rc::Rc<#inner>),
                Wrapper::Cow => quote!(::std::borrow::Cow<'static, #inner>),
            }
        }
        Shape::Collection(collection, args) => {
            let args = args.iter().map(spell).collect::<Option<Vec<_>>>()?;
            match collection {
                Collection::Vec => quote!(::std::vec::Vec<#(#args),*>),
                Collection::VecDeque => quote!(::std::collections::VecDeque<#(#args),*>),
                Collection::HashSet => quote!(::std::collections::HashSet<#(#args),*>),
                Collection::BTreeSet => quote!(::std::collections::BTreeSet<#(#args),*>),
                Collection::HashMap => quote!(::std::collections::HashMap<#(#args),*>),
                Collection::BTreeMap => quote!(::std::collections::BTreeMap<#(#args),*>),
            }
        }
        Shape::Named(path, lifetimes) => {
            let path = path_tokens(path);
            if *lifetimes == 0 {
                path
            } else {
                let lifetimes = (0..*lifetimes).map(|_| quote!('static));
                quote!(#path<#(#lifetimes),*>)
            }
        }
        Shape::Opaque(_) => return None,
    })
}

/// A listed path, from the game crate's root.
fn path_tokens(path: &str) -> TokenStream {
    let mut segments = path.split("::");
    let first = segments.next().unwrap_or_default();
    let head = if first == GAME {
        quote!(crate)
    } else {
        let first = format_ident!("{first}");
        quote!(::#first)
    };
    let rest = segments.map(|s| format_ident!("{s}"));
    quote!(#head #(::#rest)*)
}

/// Whether a value of `shape` can be built from a copy: no borrowed or shared part.
fn buildable(shape: &Shape, rule: &Rule) -> bool {
    match shape {
        Shape::Scalar(Scalar::Str) | Shape::Ref { .. } | Shape::Opaque(_) => false,
        Shape::Scalar(_) | Shape::Unit => true,
        Shape::Tuple(parts) => parts.iter().all(|p| buildable(p, rule)),
        Shape::Array(elem, _) | Shape::Option(elem) => buildable(elem, rule),
        Shape::Wrapped(Wrapper::Box, inner) => buildable(inner, rule),
        Shape::Wrapped(..) | Shape::Slice(_) | Shape::Collection(..) => false,
        Shape::Named(path, _) => {
            let Some(named) = rule.named(path) else {
                return false;
            };
            if named.foreign && named.sealed {
                return false;
            }
            rule.fields(path)
                .iter()
                .all(|f| buildable(&rule.shape(&named.module, Some(path), f.ty), rule))
        }
    }
}

fn codec_names(path: &str) -> (syn::Ident, syn::Ident) {
    let id = path.replace("::", "__");
    (format_ident!("enc__{id}"), format_ident!("dec__{id}"))
}

impl<'r> Gen<'r> {
    /// An expression of `Value` from `expr`, a reference to a copy of `shape`.
    fn encode(&mut self, shape: &Shape, expr: TokenStream) -> TokenStream {
        match shape {
            Shape::Scalar(scalar) => match scalar {
                Scalar::Bool => quote!(V::B(*#expr)),
                Scalar::Char => quote!(V::C(*#expr)),
                Scalar::Signed(_) => quote!(V::S(*#expr as i64)),
                Scalar::Unsigned(_) => quote!(V::U(*#expr as u64)),
                Scalar::Float(_) => quote!(V::F(*#expr as f64)),
                Scalar::Str | Scalar::String => {
                    quote!(V::Str(::std::string::ToString::to_string(&#expr[..])))
                }
            },
            Shape::Unit => quote!(V::None),
            Shape::Ref { to, .. } | Shape::Wrapped(_, to) => self.encode(to, quote!((&**#expr))),
            Shape::Tuple(parts) => {
                let parts: Vec<_> = parts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let i = syn::Index::from(i);
                        self.encode(p, quote!((&#expr.#i)))
                    })
                    .collect();
                quote!(V::Rec(::std::vec![#(#parts),*]))
            }
            Shape::Array(elem, _) => {
                let each = self.encode(elem, quote!(item));
                quote!(V::Seq(#expr.iter().map(|item| #each).collect()))
            }
            Shape::Option(inner) => {
                let some = self.encode(inner, quote!(item));
                quote!(match #expr { ::std::option::Option::Some(item) => V::Var(1, ::std::vec![#some]), ::std::option::Option::None => V::Var(0, ::std::vec::Vec::new()) })
            }
            Shape::Named(path, _) => {
                self.codec(path);
                let (enc, _) = codec_names(path);
                quote!(#enc(#expr))
            }
            Shape::Slice(_) | Shape::Collection(..) | Shape::Opaque(_) => {
                unreachable!("not a copy")
            }
        }
    }

    /// An expression of `shape` from `expr`, a `Value`, with `?` on a mismatch.
    fn decode(&mut self, shape: &Shape, expr: TokenStream) -> TokenStream {
        match shape {
            Shape::Scalar(scalar) => match scalar {
                Scalar::Bool => {
                    quote!(match #expr { V::B(b) => b, _ => return Err(I::changed("a bool")) })
                }
                Scalar::Char => {
                    quote!(match #expr { V::C(c) => c, _ => return Err(I::changed("a char")) })
                }
                Scalar::Signed(_) | Scalar::Unsigned(_) => quote!(I::int(#expr)?),
                Scalar::Float(name) => {
                    let name = format_ident!("{name}");
                    quote!(match #expr { V::F(f) => f as #name, _ => return Err(I::changed("a float")) })
                }
                Scalar::String => {
                    quote!(match #expr { V::Str(s) => s, _ => return Err(I::changed("a string")) })
                }
                Scalar::Str => unreachable!("not buildable"),
            },
            Shape::Unit => quote!(()),
            Shape::Wrapped(Wrapper::Box, inner) => {
                let inner = self.decode(inner, expr);
                quote!(::std::boxed::Box::new(#inner))
            }
            Shape::Tuple(parts) => {
                let names: Vec<_> = (0..parts.len()).map(|i| format_ident!("f{i}")).collect();
                let n = parts.len();
                let parts: Vec<_> = parts
                    .iter()
                    .zip(&names)
                    .map(|(p, name)| self.decode(p, quote!(#name)))
                    .collect();
                quote!({ let [#(#names),*] = I::fields::<_, #n>(I::rec(#expr)?)?; (#(#parts,)*) })
            }
            Shape::Array(elem, n) => {
                let each = self.decode(elem, quote!(item));
                quote!({
                    let items = I::fields::<_, #n>(I::seq(#expr)?)?;
                    let mut out = ::std::vec::Vec::with_capacity(#n);
                    for item in items { out.push(#each); }
                    match <[_; #n]>::try_from(out) { Ok(a) => a, Err(_) => unreachable!() }
                })
            }
            Shape::Option(inner) => {
                let some = self.decode(inner, quote!(item));
                quote!(match #expr {
                    V::Var(0, _) => ::std::option::Option::None,
                    V::Var(1, items) => { let [item] = I::fields::<_, 1>(items)?; ::std::option::Option::Some(#some) }
                    _ => return Err(I::changed("an option")),
                })
            }
            Shape::Named(path, _) => {
                self.codec(path);
                let (_, dec) = codec_names(path);
                quote!(#dec(#expr)?)
            }
            _ => unreachable!("not buildable"),
        }
    }

    /// The codec of the listed copy type at `path`, once.
    fn codec(&mut self, path: &str) {
        if self.codecs.contains_key(path) {
            return;
        }
        let rule = self.rule;
        let build = buildable(&Shape::Named(path.into(), 0), rule);
        self.codecs.insert(path.to_owned(), build);
        let named = rule.named(path).expect("listed");
        let ty = spell(&Shape::Named(path.into(), named.lifetimes)).expect("named");
        let fields = rule.fields(path);
        let shapes: Vec<Shape> = fields
            .iter()
            .map(|f| rule.shape(&named.module, Some(path), f.ty))
            .collect();
        let (enc, dec) = codec_names(path);
        let path_ty = path_tokens(path);
        let (encode, decode) = match &named.item {
            syn::Item::Struct(item) => {
                let binds: Vec<_> = (0..fields.len()).map(|i| format_ident!("f{i}")).collect();
                let members: Vec<_> = fields.iter().map(|f| &f.member).collect();
                let pattern = quote!(#path_ty { #(#members: #binds,)* .. });
                let encoded: Vec<_> = shapes
                    .iter()
                    .zip(&binds)
                    .map(|(s, b)| self.encode(s, quote!(#b)))
                    .collect();
                let encode = quote!({ let #pattern = value; V::Rec(::std::vec![#(#encoded),*]) });
                let n = fields.len();
                let decoded: Vec<_> = shapes
                    .iter()
                    .zip(&binds)
                    .map(|(s, b)| {
                        if build {
                            self.decode(s, quote!(#b))
                        } else {
                            quote!()
                        }
                    })
                    .collect();
                let decode = quote!({
                    let [#(#binds),*] = I::fields::<_, #n>(I::rec(value)?)?;
                    Ok(#path_ty { #(#members: #decoded),* })
                });
                let _ = item;
                (encode, decode)
            }
            syn::Item::Enum(item) => {
                let mut encode_arms = Vec::new();
                let mut decode_arms = Vec::new();
                for (case, variant) in item.variants.iter().enumerate() {
                    let case = case as u32;
                    let ident = &variant.ident;
                    let at: Vec<usize> = fields
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| f.case.map(|(c, _)| c) == Some(case))
                        .map(|(i, _)| i)
                        .collect();
                    let binds: Vec<_> = at.iter().map(|i| format_ident!("f{i}")).collect();
                    let members: Vec<_> = at.iter().map(|i| &fields[*i].member).collect();
                    let pattern = quote!(#path_ty::#ident { #(#members: #binds,)* .. });
                    let encoded: Vec<_> = at
                        .iter()
                        .zip(&binds)
                        .map(|(i, b)| self.encode(&shapes[*i], quote!(#b)))
                        .collect();
                    encode_arms
                        .push(quote!(#pattern => V::Var(#case, ::std::vec![#(#encoded),*]),));
                    if build {
                        let decoded: Vec<_> = at
                            .iter()
                            .zip(&binds)
                            .map(|(i, b)| self.decode(&shapes[*i], quote!(#b)))
                            .collect();
                        let n = at.len();
                        decode_arms.push(quote!(V::Var(#case, items) => {
                            let [#(#binds),*] = I::fields::<_, #n>(items)?;
                            Ok(#path_ty::#ident { #(#members: #decoded),* })
                        }));
                    }
                }
                (
                    // Another crate's `#[non_exhaustive]` enum needs a wildcard; every case of this
                    // build is listed, so it never matches.
                    quote!(match value { #(#encode_arms)* _ => V::None }),
                    quote!(match value { #(#decode_arms)* _ => Err(I::changed("a case")) }),
                )
            }
            _ => unreachable!("a struct or an enum"),
        };
        self.items.push(quote! {
            #[allow(unused_variables, unreachable_patterns, clippy::all)]
            fn #enc(value: &#ty) -> V { #encode }
        });
        if build {
            self.items.push(quote! {
                #[allow(unused_variables, clippy::all)]
                fn #dec(value: V) -> ::std::result::Result<#ty, I::Refusal> { #decode }
            });
        }
    }

    /// The id of the walked type `shape`, generating its node function once. Reference and
    /// wrapper layers are stepped through when a node is made, so `shape` is never one.
    fn node(&mut self, shape: &Shape) -> u32 {
        let ty = spell(shape).expect("crossable").to_string();
        if let Some(id) = self.ids.get(&ty) {
            return *id;
        }
        let id = self.nodes.len() as u32;
        self.ids.insert(ty.clone(), id);
        self.nodes.push((ty, TokenStream::new()));
        let body = self.node_body(shape);
        let name = format_ident!("node_{id}");
        self.items.push(quote! {
            #[allow(unused_variables, unreachable_patterns, unreachable_code, clippy::all)]
            unsafe fn #name(p: *mut (), w: bool, op: I::Op<'_>) -> ::std::result::Result<I::Reply, I::Refusal> {
                unsafe { #body }
            }
        });
        self.nodes[id as usize].1 = quote!(#name);
        id
    }

    /// `Reply::Node` for the value of `shape` that `q` (a `*mut ()`) points to, stepping through
    /// references and wrappers: a node may be written only if `w` and every layer allows it, and
    /// a node that may not is never reached through a `&mut`.
    fn step(&mut self, shape: &Shape, q: TokenStream, w: TokenStream) -> TokenStream {
        let through = |this: &mut Self, ty: TokenStream, to: &Shape, mutable: bool, owned: bool| {
            let inner = spell(to).expect("crossable");
            let next = if owned || mutable {
                quote!(if w { &mut **(q as *mut #ty) as *mut #inner as *mut () } else { &**(q as *const #ty) as *const #inner as *mut () })
            } else {
                quote!(&**(q as *const #ty) as *const #inner as *mut ())
            };
            let w_next = if owned || mutable {
                quote!(w)
            } else {
                quote!(false)
            };
            let rest = this.step(to, quote!(q), w_next);
            quote!({ let (q, w): (*mut (), bool) = (#q, #w); let q = #next; #rest })
        };
        match shape {
            Shape::Ref { mutable, to } => {
                let ty = spell(shape).expect("crossable");
                through(self, ty, to, *mutable, false)
            }
            Shape::Wrapped(wrapper, to) => {
                let ty = spell(shape).expect("crossable");
                through(self, ty, to, false, *wrapper == Wrapper::Box)
            }
            _ => {
                let id = self.node(shape);
                quote!(Ok(I::Reply::Node(#q, #id, #w)))
            }
        }
    }

    /// The arms for one part numbered `i`: `get` reaches it as `&T`, `set` as `&mut T`, and
    /// `ptr` as a `*mut ()` made through `&mut` only if `w`; each may `return` a refusal.
    fn part(
        &mut self,
        i: u32,
        shape: &Shape,
        get: TokenStream,
        set: TokenStream,
        ptr: TokenStream,
    ) -> TokenStream {
        match self.rule.crossing_of(shape) {
            Crossing::Opaque => quote!(),
            Crossing::Copy(_) => {
                let read = self.encode(shape, quote!(r));
                let write = if buildable(shape, self.rule) {
                    let value = self.decode(shape, quote!(value));
                    quote!(I::Op::Set(#i, value) => {
                        if !w { return Err(I::not_here("a read-only node")); }
                        let value = #value;
                        let r = #set;
                        *r = value;
                        Ok(I::Reply::Done)
                    })
                } else {
                    quote!()
                };
                quote! {
                    I::Op::Get(#i) => { let r = #get; Ok(I::Reply::Value(#read)) }
                    #write
                }
            }
            Crossing::Handle => {
                let step = self.step(shape, quote!(#ptr), quote!(w));
                quote!(I::Op::Field(#i) => #step,)
            }
        }
    }

    /// `get`, `set` and `ptr` for a part matched out of `this` by `pattern` (binding `field`).
    fn matched(
        rust: &TokenStream,
        pattern: TokenStream,
    ) -> (TokenStream, TokenStream, TokenStream) {
        let miss = quote!(_ => return Err(I::not_here("another case")));
        (
            quote!(match &*(p as *const #rust) { #pattern => field, #miss }),
            quote!(match &mut *(p as *mut #rust) { #pattern => field, #miss }),
            quote!(if w {
                match &mut *(p as *mut #rust) { #pattern => field as *mut _ as *mut (), #miss }
            } else {
                match &*(p as *const #rust) { #pattern => field as *const _ as *mut (), #miss }
            }),
        )
    }

    fn node_body(&mut self, shape: &Shape) -> TokenStream {
        let rule = self.rule;
        let rust = spell(shape).expect("crossable");
        let read = match rule.crossing_of(shape) {
            Crossing::Copy(_) => {
                let read = self.encode(shape, quote!(r));
                quote!(I::Op::Read => { let r = &*(p as *const #rust); Ok(I::Reply::Value(#read)) })
            }
            _ => quote!(),
        };
        let len =
            quote!(I::Op::Len => Ok(I::Reply::Value(V::U((&*(p as *const #rust)).len() as u64))),);
        let arms = match shape {
            Shape::Named(path, _) => {
                let named = rule.named(path).expect("listed");
                let path_ty = path_tokens(path);
                let mut arms = Vec::new();
                for (i, field) in rule.fields(path).iter().enumerate() {
                    let part_shape = rule.shape(&named.module, Some(path), field.ty);
                    let member = &field.member;
                    let (get, set, ptr) = match field.case {
                        None => (
                            quote!(&(*(p as *const #rust)).#member),
                            quote!(&mut (*(p as *mut #rust)).#member),
                            quote!(&raw mut (*(p as *mut #rust)).#member as *mut ()),
                        ),
                        Some((_, case)) => {
                            Self::matched(&rust, quote!(#path_ty::#case { #member: field, .. }))
                        }
                    };
                    arms.push(self.part(i as u32, &part_shape, get, set, ptr));
                }
                let case = match &named.item {
                    syn::Item::Enum(item) => {
                        let cases = item.variants.iter().enumerate().map(|(i, v)| {
                            let ident = &v.ident;
                            let i = i as u64;
                            quote!(#path_ty::#ident { .. } => #i,)
                        });
                        quote!(I::Op::Case => Ok(I::Reply::Value(V::U(match &*(p as *const #rust) { #(#cases)* _ => u64::MAX }))),)
                    }
                    _ => quote!(),
                };
                quote!(#(#arms)* #case)
            }
            Shape::Tuple(parts) => {
                let mut arms = Vec::new();
                for (i, part) in parts.iter().enumerate() {
                    let index = syn::Index::from(i);
                    arms.push(self.part(
                        i as u32,
                        part,
                        quote!(&(*(p as *const #rust)).#index),
                        quote!(&mut (*(p as *mut #rust)).#index),
                        quote!(&raw mut (*(p as *mut #rust)).#index as *mut ()),
                    ));
                }
                quote!(#(#arms)*)
            }
            Shape::Option(inner) => {
                let (get, set, ptr) =
                    Self::matched(&rust, quote!(::std::option::Option::Some(field)));
                let some = self.part(0, inner, get, set, ptr);
                quote! {
                    I::Op::Case => Ok(I::Reply::Value(V::U((&*(p as *const #rust)).is_some() as u64))),
                    #some
                }
            }
            Shape::Array(elem, _) | Shape::Slice(elem) => {
                let index = self.indexed(&rust, elem, quote!(get(i)), quote!(get_mut(i)));
                quote!(#len #index)
            }
            Shape::Collection(collection, args) => match collection {
                Collection::Vec | Collection::VecDeque => {
                    let index = self.indexed(&rust, &args[0], quote!(get(i)), quote!(get_mut(i)));
                    quote!(#len #index)
                }
                Collection::HashMap | Collection::BTreeMap if buildable(&args[0], rule) => {
                    let key = self.decode(&args[0], quote!(key.clone()));
                    let entry = self.element(&args[1]);
                    quote! {
                        #len
                        I::Op::Key(key) => {
                            let key = #key;
                            let item: ::std::option::Option<*mut ()> = if w {
                                (&mut *(p as *mut #rust)).get_mut(&key).map(|item| item as *mut _ as *mut ())
                            } else {
                                (&*(p as *const #rust)).get(&key).map(|item| item as *const _ as *mut ())
                            };
                            match item { Some(q) => #entry, None => Err(I::not_here("no such key")) }
                        }
                    }
                }
                _ => len,
            },
            _ => quote!(),
        };
        quote!(match op { #arms #read _ => Err(I::not_here("not on this node")) })
    }

    /// An element's reply, from `q`, a `*mut ()` to it: its node, so it can be written field by
    /// field, or a scalar's copy.
    fn element(&mut self, shape: &Shape) -> TokenStream {
        match self.rule.crossing_of(shape) {
            Crossing::Copy(_) if matches!(shape, Shape::Named(..) | Shape::Tuple(_)) => {
                self.step(shape, quote!(q), quote!(w))
            }
            Crossing::Copy(_) => {
                let rust = spell(shape).expect("crossable");
                let value = self.encode(shape, quote!(r));
                quote!({ let r = &*(q as *const #rust); Ok(I::Reply::Value(#value)) })
            }
            Crossing::Handle => self.step(shape, quote!(q), quote!(w)),
            Crossing::Opaque => quote!(Err(I::not_here("an element that can't cross"))),
        }
    }

    fn indexed(
        &mut self,
        rust: &TokenStream,
        elem: &Shape,
        get: TokenStream,
        get_mut: TokenStream,
    ) -> TokenStream {
        let element = self.element(elem);
        quote! {
            I::Op::Index(i) => {
                let i = usize::try_from(i).unwrap_or(usize::MAX);
                let item: ::std::option::Option<*mut ()> = if w {
                    (&mut *(p as *mut #rust)).#get_mut.map(|item| item as *mut _ as *mut ())
                } else {
                    (&*(p as *const #rust)).#get.map(|item| item as *const _ as *mut ())
                };
                match item { Some(q) => #element, None => Err(I::not_here("an index past the end")) }
            }
        }
    }

    /// The thunk of the listed function `internal`, if every part of it can cross.
    fn thunk(&mut self, internal: &Internal) -> Option<TokenStream> {
        let rule = self.rule;
        self.skip_reason = "signature, generic impl, or unsafe/async function";
        let sig = signature(&internal.signature)?;
        if sig.generics.params.len() != sig.generics.lifetimes().count()
            || sig.asyncness.is_some()
            || sig.unsafety.is_some()
            || sig.variadic.is_some()
        {
            return None;
        }
        self.skip_reason = "unresolved or opaque owner";
        let path = bare(&internal.path);
        let (owner, name) = path.rsplit_once("::")?;
        let name = format_ident!("{name}");
        let (module, this) = match internal.kind {
            "method" => {
                let (module, ty) = owner.rsplit_once("::")?;
                let written: syn::Path = syn::parse_str(ty).ok()?;
                let ty = match rule.shape(
                    module,
                    None,
                    &syn::Type::Path(syn::TypePath {
                        qself: None,
                        path: written,
                    }),
                ) {
                    Shape::Named(path, _) => path,
                    _ => return None,
                };
                (module.to_owned(), Some(ty))
            }
            "fn" => (owner.to_owned(), None),
            _ => return None,
        };
        self.skip_reason = "static or bounded input lifetime";
        struct StaticInput(bool);
        impl<'ast> syn::visit::Visit<'ast> for StaticInput {
            fn visit_lifetime(&mut self, lifetime: &'ast syn::Lifetime) {
                self.0 |= lifetime.ident == "static";
            }
        }
        let mut static_input = StaticInput(false);
        for input in &sig.inputs {
            syn::visit::Visit::visit_fn_arg(&mut static_input, input);
        }
        if static_input.0
            || sig.generics.where_clause.is_some()
            || sig.generics.lifetimes().any(|l| !l.bounds.is_empty())
        {
            return None;
        }
        let mut writes = 0u64;
        let mut receiver = quote!();
        let mut inputs = sig.inputs.iter();
        if let Some(this) = &this {
            let Some(syn::FnArg::Receiver(r)) = inputs.next() else {
                // An associated function: called on the type, no receiver.
                return self.thunk_tail(
                    internal,
                    &module,
                    Some(this),
                    None,
                    sig.inputs.iter(),
                    &sig.output,
                    0,
                    &name,
                );
            };
            self.skip_reason = "by-value receiver";
            r.reference.as_ref()?;
            let ty = spell(&Shape::Named(this.clone(), rule.named(this)?.lifetimes))?;
            if r.mutability.is_some() {
                writes |= 1;
                receiver = quote!(&mut *(this.ok_or(I::changed("a receiver"))? as *mut #ty),);
            } else {
                receiver = quote!(&*(this.ok_or(I::changed("a receiver"))? as *const #ty),);
            }
        }
        self.thunk_tail(
            internal,
            &module,
            this.as_deref(),
            Some(receiver),
            inputs,
            &sig.output,
            writes,
            &name,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn thunk_tail<'a>(
        &mut self,
        internal: &Internal,
        module: &str,
        this: Option<&str>,
        receiver: Option<TokenStream>,
        inputs: impl Iterator<Item = &'a syn::FnArg>,
        output: &syn::ReturnType,
        mut writes: u64,
        name: &syn::Ident,
    ) -> Option<TokenStream> {
        let rule = self.rule;
        let mut lets = Vec::new();
        let mut args = Vec::new();
        // Named input lifetimes can be tied to a receiver's stored borrows. Spelling a listed
        // lifetime-bearing input with `'static` would also invent an outlives guarantee.
        struct InputLifetimes<'r> {
            rule: &'r Rule,
            module: &'r str,
            this: Option<&'r str>,
            forbidden: bool,
        }
        impl<'ast> syn::visit::Visit<'ast> for InputLifetimes<'_> {
            fn visit_lifetime(&mut self, lifetime: &'ast syn::Lifetime) {
                self.forbidden |= lifetime.ident != "_";
            }

            fn visit_type(&mut self, ty: &'ast syn::Type) {
                if matches!(self.rule.shape(self.module, self.this, ty), Shape::Named(_, n) if n > 0)
                {
                    self.forbidden = true;
                }
                syn::visit::visit_type(self, ty);
            }
        }
        self.skip_reason = "argument cannot cross or be built";
        for (i, input) in inputs.enumerate() {
            let syn::FnArg::Typed(input) = input else {
                return None;
            };
            let mut lifetimes = InputLifetimes {
                rule,
                module,
                this,
                forbidden: false,
            };
            syn::visit::Visit::visit_type(&mut lifetimes, &input.ty);
            if lifetimes.forbidden {
                self.skip_reason = "named input lifetime or lifetime-bearing input type";
                return None;
            }
            let shape = rule.shape(module, this, &input.ty);
            let arg = format_ident!("a{i}");
            match (&shape, rule.crossing_of(&shape)) {
                (_, Crossing::Opaque) => return None,
                (Shape::Ref { mutable, to }, Crossing::Handle)
                | (Shape::Ref { mutable, to }, Crossing::Copy(_))
                    if *mutable || rule.crossing_of(to) == Crossing::Handle =>
                {
                    let ty = spell(to)?;
                    if *mutable {
                        writes |= 1 << (i + 1);
                        lets.push(quote!(let #arg = &mut *(I::node(#arg)? as *mut #ty);));
                    } else {
                        lets.push(quote!(let #arg = &*(I::node(#arg)? as *const #ty);));
                    }
                    args.push(quote!(#arg));
                }
                (Shape::Ref { to, .. }, Crossing::Copy(_)) => {
                    if matches!(**to, Shape::Scalar(Scalar::Str)) {
                        let value =
                            self.decode(&Shape::Scalar(Scalar::String), quote!(I::value(#arg)?));
                        lets.push(quote!(let #arg: ::std::string::String = #value;));
                        args.push(quote!(&#arg[..]));
                    } else if buildable(to, rule) {
                        let value = self.decode(to, quote!(I::value(#arg)?));
                        lets.push(quote!(let #arg = #value;));
                        args.push(quote!(&#arg));
                    } else {
                        return None;
                    }
                }
                (_, Crossing::Copy(_)) if buildable(&shape, rule) => {
                    let value = self.decode(&shape, quote!(I::value(#arg)?));
                    lets.push(quote!(let #arg = #value;));
                    args.push(quote!(#arg));
                }
                _ => return None,
            }
        }
        let n = args.len();
        let names: Vec<_> = (0..n).map(|i| format_ident!("a{i}")).collect();
        self.skip_reason = "result cannot cross";
        let result = match output {
            syn::ReturnType::Default => quote!(Ok(I::Reply::Done)),
            syn::ReturnType::Type(_, ty) => {
                let shape = rule.shape(module, this, ty);
                match (&shape, rule.crossing_of(&shape)) {
                    (_, Crossing::Opaque) => return None,
                    (_, Crossing::Copy(_)) => {
                        let value = self.encode(&shape, quote!((&result)));
                        quote!(Ok(I::Reply::Value(#value)))
                    }
                    (Shape::Ref { .. }, Crossing::Handle) => {
                        let node =
                            self.step(&shape, quote!(&raw mut result as *mut ()), quote!(true));
                        quote!({ let mut result = result; #node })
                    }
                    _ => return None,
                }
            }
        };
        let callee = match this {
            Some(this) => {
                let ty = path_tokens(this);
                quote!(#ty::#name)
            }
            None => {
                let owner = bare(&internal.path);
                path_tokens(&owner)
            }
        };
        let receiver = receiver.unwrap_or_default();
        let thunk = format_ident!("thunk_{}", self.items.len());
        let path = &internal.path;
        self.items.push(quote! {
            #[allow(unused_variables, unused_mut, clippy::all)]
            unsafe fn #thunk(this: ::std::option::Option<*mut ()>, args: ::std::vec::Vec<I::Arg>) -> ::std::result::Result<I::Reply, I::Refusal> {
                unsafe {
                    let [#(#names),*] = I::fields::<_, #n>(args)?;
                    #(#lets)*
                    let result = #callee(#receiver #(#args),*);
                    #result
                }
            }
        });
        Some(quote!(I::Thunk { path: #path, writes: #writes, call: #thunk },))
    }
}

/// The manifest records impl bounds as well as the method signature. A bounded or type-generic
/// impl is not callable on every receiver, even when the method itself has no generics.
fn signature(written: &str) -> Option<syn::Signature> {
    use syn::parse::Parser;
    let parser = |input: syn::parse::ParseStream<'_>| {
        if input.peek(syn::Token![impl]) {
            input.parse::<syn::Token![impl]>()?;
            let generics: syn::Generics = input.parse()?;
            let _: syn::Type = input.parse()?;
            let bounds: Option<syn::WhereClause> = input.parse()?;
            if generics.params.len() != generics.lifetimes().count()
                || generics.lifetimes().any(|l| !l.bounds.is_empty())
                || bounds.is_some()
            {
                return Err(input.error("a generic or bounded impl"));
            }
            let content;
            syn::braced!(content in input);
            content.parse::<syn::Signature>()
        } else {
            input.parse::<syn::Signature>()
        }
    };
    parser.parse_str(written).ok()
}

/// The `__internals` module for `internals`, and what it holds.
pub fn table(internals: &[Internal]) -> (String, Counts) {
    let rule = Rule::new(internals);
    let mut generator = Gen {
        rule: &rule,
        ids: HashMap::new(),
        nodes: Vec::new(),
        codecs: HashMap::new(),
        items: Vec::new(),
        skip_reason: "unknown",
    };
    let mut paths: Vec<&str> = rule.paths().collect();
    paths.sort_unstable();
    for path in paths {
        let shape = Shape::Named(path.to_owned(), rule.named(path).expect("listed").lifetimes);
        if rule.crossing_of(&shape) != Crossing::Opaque {
            generator.node(&shape);
        }
    }
    let mut thunks = Vec::new();
    let mut counts = Counts {
        skipped_types: rule.skipped_types(),
        ..Counts::default()
    };
    for internal in internals {
        if !matches!(internal.kind, "method" | "fn") {
            continue;
        }
        if !internal.cfg.is_empty() {
            counts.skipped += 1;
            *counts
                .skipped_functions
                .entry("conditional function")
                .or_default() += 1;
            continue;
        }
        match generator.thunk(internal) {
            Some(thunk) => thunks.push(thunk),
            None => {
                counts.skipped += 1;
                *counts
                    .skipped_functions
                    .entry(generator.skip_reason)
                    .or_default() += 1;
            }
        }
    }
    counts.types = generator.nodes.len();
    counts.thunks = thunks.len();
    let types = generator
        .nodes
        .iter()
        .map(|(ty, node)| quote!((#ty, #node),));
    let items = &generator.items;
    let code = quote! {
        /// The game's internals for the wasm runtime (#589 W): generated by the staging pass.
        #[doc(hidden)]
        #[allow(unsafe_code, unused_unsafe, unused_parens, non_snake_case, dead_code)]
        pub mod __internals {
            use ::mod_points::internals as I;
            use I::Value as V;
            #(#items)*
            /// The table.
            pub static TABLE: I::Table = I::Table {
                types: &[#(#types)*],
                thunks: &[#(#thunks)*],
            };
        }
    };
    (code.to_string(), counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crossing::tests::internal;

    #[test]
    fn generated_collection_nodes_compile_and_read_the_actual_element() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let build = std::env::var_os("CARGO_TARGET_DIR")
            .map_or_else(|| root.join("target"), std::path::PathBuf::from)
            .join(format!("internals-table-fixture-{}", std::process::id()));
        std::fs::create_dir_all(&build).unwrap();
        let runtime = root
            .join("crates/horizon-mod-points/src/internals.rs")
            .canonicalize()
            .unwrap();
        let (code, _) = table(&[internal(
            "horizon_client::Bag",
            "struct",
            "struct Bag { items: Vec<u8> }",
        )]);
        let source = format!(
            r#"
            extern crate self as mod_points;
            #[path = {runtime:?}] pub mod internals;
            struct Bag {{ items: Vec<u8> }}
            {code}
            fn main() {{
                use internals::{{Op, Reply, Value}};
                let mut bag = Bag {{ items: vec![7] }};
                let (_, walk) = __internals::TABLE.types.iter().find(|(name, _)| name.ends_with("Bag")).unwrap();
                unsafe {{
                    let Reply::Node(p, ty, writable) = walk(&raw mut bag as *mut (), false, Op::Field(0)).unwrap() else {{ panic!("not a node") }};
                    assert!(!writable);
                    assert_eq!(__internals::TABLE.types[ty as usize].1(p, writable, Op::Index(0)), Ok(Reply::Value(Value::U(7))));
                }}
            }}
        "#
        );
        let input = build.join("main.rs");
        let output = build.join(format!("fixture{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(&input, source).unwrap();
        let compiled = std::process::Command::new("rustc")
            .args(["--edition=2024", "-Dwarnings"])
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        assert!(
            std::process::Command::new(output)
                .status()
                .unwrap()
                .success()
        );
        std::fs::remove_dir_all(build).unwrap();
    }

    #[test]
    fn method_signatures_accept_bare_and_impl_forms_but_not_bounded_impls() {
        for written in [
            "fn count(&self) -> usize",
            "impl Game<'_> { fn count(&self) -> usize }",
        ] {
            let (_, counts) = table(&[
                internal(
                    "horizon_client::Game",
                    "struct",
                    "struct Game<'a> { label: &'a str }",
                ),
                internal("horizon_client::Game<'_>::count", "method", written),
            ]);
            assert_eq!(counts.thunks, 1, "{written}");
        }
        assert!(signature("impl<T> Game<T> { fn count(&self) -> usize }").is_none());
        assert!(signature("impl<'a: 'static> Game<'a> { fn count(&self) -> usize }").is_none());
        assert!(signature("impl Game where Self: Sized { fn count(&self) -> usize }").is_none());
    }

    #[test]
    fn a_call_cannot_keep_a_decoded_borrow_beyond_its_frame() {
        let (_, counts) = table(&[
            internal(
                "horizon_client::take_static",
                "fn",
                "fn take_static(s: &'static str)",
            ),
            internal(
                "horizon_client::take_bounded",
                "fn",
                "fn take_bounded<'a: 'static>(s: &'a str)",
            ),
            internal("horizon_client::take", "fn", "fn take(s: &str)"),
        ]);
        assert_eq!(counts.thunks, 1);
        assert_eq!(
            counts.skipped_functions["static or bounded input lifetime"],
            2
        );
    }

    #[test]
    fn an_argument_cannot_supply_a_borrow_stored_in_the_receiver() {
        let (_, counts) = table(&[
            internal(
                "horizon_client::World",
                "struct",
                "struct World { rows: Vec<u8> }",
            ),
            internal(
                "horizon_client::Holder",
                "struct",
                "struct Holder<'a> { world: &'a World }",
            ),
            internal(
                "horizon_client::Foo",
                "struct",
                "struct Foo<'a> { world: &'a World }",
            ),
            internal(
                "horizon_client::Holder<'_>::replace",
                "method",
                "impl<'a> Holder<'a> { fn replace(&mut self, world: &'a World) }",
            ),
            internal("horizon_client::take", "fn", "fn take(foo: &mut Foo<'_>)"),
            internal("horizon_client::inspect", "fn", "fn inspect(world: &World)"),
        ]);
        assert_eq!(counts.thunks, 1);
        assert_eq!(
            counts.skipped_functions["named input lifetime or lifetime-bearing input type"],
            2
        );
    }

    #[test]
    fn conditional_layouts_and_unwidened_lower_layouts_stay_opaque() {
        let (code, counts) = table(&[
            internal(
                "horizon_client::Conditional",
                "struct",
                "struct Conditional { #[cfg(feature = \"audio\")] sound: u8 }",
            ),
            internal(
                "horizon_client::Cases",
                "enum",
                "enum Cases { #[cfg(test)] Test, Normal }",
            ),
            internal(
                "horizon_client_core::External",
                "struct",
                "struct External { private: u8 }",
            ),
            internal(
                "mod_api::External",
                "struct",
                "struct External { private: u8 }",
            ),
            internal(
                "mod_points::External",
                "struct",
                "struct External { private: u8 }",
            ),
            internal(
                "horizon_client::Generic",
                "struct",
                "struct Generic<T> { value: T }",
            ),
        ]);
        assert_eq!(counts.types, 0, "{code}");
        assert_eq!(counts.skipped_types["conditional field or variant"], 2);
        assert_eq!(counts.skipped_types["lower-crate layout not widened"], 3);
        assert_eq!(counts.skipped_types["generic type"], 1);
    }

    #[test]
    fn reachable_lower_shapes_are_walked_and_sealed_ones_never_built() {
        let shape = |path: &str, kind: &'static str, signature: &str| {
            let mut entry = internal(path, kind, signature);
            entry.shape_only = true;
            entry
        };
        let (code, counts) = table(&[
            shape(
                "horizon_items::Stack",
                "struct",
                "struct Stack { count : u8 , }",
            ),
            shape(
                "horizon_items::Sealed",
                "struct",
                "# [non_exhaustive] struct Sealed { count : u8 , }",
            ),
            shape(
                "horizon_items::Kind",
                "enum",
                "enum Kind { # [non_exhaustive] One { n : u8 } , Two , }",
            ),
            shape("mod_api::Api", "struct", "struct Api { private : u8 , }"),
        ]);
        syn::parse_file(&code).unwrap();
        assert_eq!(counts.types, 3, "{code}");
        assert_eq!(counts.skipped_types["lower-crate layout not widened"], 1);
        assert!(code.contains("fn dec__horizon_items__Stack"));
        assert!(!code.contains("fn dec__horizon_items__Sealed"));
        assert!(!code.contains("fn dec__horizon_items__Kind"));
        // Read from another crate: patterns that tolerate fields and cases they don't list.
        assert!(code.contains("count : f0 , .."), "{code}");
    }

    #[test]
    fn import_facts_recover_crossable_functions_without_global_name_guesses() {
        let mut take = internal("horizon_client::b::take", "fn", "fn take(data: Alias)");
        take.refs
            .insert("Alias".into(), "horizon_client::a::Data".into());
        let (_, counts) = table(&[
            internal("horizon_client::a::Data", "struct", "struct Data { x: u8 }"),
            take,
        ]);
        assert_eq!(counts.thunks, 1);
    }

    #[test]
    fn unresolved_imports_and_unexpected_generics_do_not_borrow_another_types_layout() {
        let (_, counts) = table(&[
            internal("horizon_client::a::Cell", "struct", "struct Cell { x: u8 }"),
            internal("horizon_client::a::take", "fn", "fn take(c: Cell<u64>)"),
            internal("horizon_client::b::take", "fn", "fn take(c: Cell)"),
        ]);
        assert_eq!(counts.types, 1);
        assert_eq!(counts.thunks, 0);
        assert_eq!(counts.skipped, 2);
    }

    #[test]
    fn unsized_and_custom_hasher_fields_are_not_walked() {
        let (code, counts) = table(&[internal(
            "horizon_client::Fields",
            "struct",
            "struct Fields { slice: &'static [u8], map: HashMap<String, u8, UnknownHasher>, tuple: (u8, Unknown) }",
        )]);
        assert_eq!(counts.types, 1);
        assert!(!code.contains("I :: Op :: Field"));
    }

    #[test]
    fn the_table_covers_what_crosses_and_parses() {
        let internals = [
            internal(
                "horizon_client::g::Pos",
                "struct",
                "struct Pos { x : i32 , y : i32 , z : i32 , }",
            ),
            internal(
                "horizon_client::g::Mode",
                "enum",
                "enum Mode { Survival , Custom (u8) , Named { name : String } , }",
            ),
            internal(
                "horizon_client::g::Player",
                "struct",
                "struct Player { pos : Pos , mode : Mode , tag : & 'static str , }",
            ),
            internal(
                "horizon_client::g::World",
                "struct",
                "struct World { players : Vec < Player > , spawn : Option < Pos > , by_name : HashMap < String , Player > , lock : RefCell < u8 > , }",
            ),
            internal(
                "horizon_client::g::Game",
                "struct",
                "struct Game < 'a > { world : Box < World > , shared : Arc < World > , res : & 'a World , }",
            ),
            internal(
                "horizon_client::g::Game<'_>::spawn",
                "method",
                "fn spawn (& mut self , at : Pos , name : & str) -> Option < & mut Player >",
            ),
            internal(
                "horizon_client::g::Game<'_>::count",
                "method",
                "fn count (& self) -> usize",
            ),
            internal(
                "horizon_client::g::Game<'_>::world",
                "method",
                "fn world (& self) -> & World",
            ),
            internal(
                "horizon_client::g::Game<'_>::closure",
                "method",
                "fn closure (& self , f : impl Fn ()) -> usize",
            ),
            internal(
                "horizon_client::g::Game<'_>::generic",
                "method",
                "fn generic < T > (& self , t : T)",
            ),
            internal(
                "horizon_client::g::Game<'_>::owned",
                "method",
                "fn owned (& self) -> Vec < Player >",
            ),
            internal(
                "horizon_client::g::helper",
                "fn",
                "fn helper (world : & mut World , pos : [i32 ; 3]) -> bool",
            ),
        ];
        let (code, counts) = table(&internals);
        syn::parse_file(&code).unwrap_or_else(|e| panic!("{e}\n{code}"));
        // Pos, Mode, Player, World, Game, and World's list and map; `Option<Pos>` is a copy.
        assert_eq!(counts.types, 7, "{code}");
        assert_eq!(counts.thunks, 4);
        assert_eq!(counts.skipped, 3);
        // A borrowed field is read, never written; a shared one is walked read-only.
        assert!(code.contains("fn enc__horizon_client__g__Player"));
        assert!(!code.contains("fn dec__horizon_client__g__Player"));
        assert!(code.contains("fn dec__horizon_client__g__Pos"));
    }
}
