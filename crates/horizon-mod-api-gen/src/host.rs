use super::*;

/// The Backend reads a mob's goal tick answers from its read view (#1077): the
/// ticking mob itself, the player the tick sees, and the tick's snapshot of the
/// other entities. The generated view forwards these to the engine's
/// `GoalCx::r_*` methods and refuses every other backend read its typed
/// refusal; the handle docs of these carry the snapshot rule.
pub(super) const GOAL_VIEW_READS: &[&str] = &[
    "entity_position",
    "entity_block_position",
    "entity_velocity",
    "entity_rotation",
    "entity_head_yaw",
    "entity_bounding_box",
    "entity_eye_height",
    "entity_eye_position",
    "entity_distance_to_sqr",
    "entity_on_ground",
    "entity_kind",
    "living_health",
    "living_max_health",
    "living_alive",
];

pub(super) fn generate(
    root: &Path,
    catalog: &Catalog,
    code: &str,
) -> Result<(String, String, String), String> {
    let contexts = catalog.draw_contexts.as_slice();
    let api_source = fs::read_to_string(root.join("crates/horizon-mod-api/src/lib.rs"))
        .map_err(|e| e.to_string())?;
    let types = api_types(&api_source, code)?;
    let mut game = Vec::new();
    game_methods(
        &root.join("crates/horizon-client/src/game"),
        "horizon_client::game",
        &mut game,
    )?;
    let mut host_traits = String::new();
    let mut host_impl = String::new();
    let mut server_traits = String::new();
    let mut server_impl = String::new();
    let mut client_traits = String::new();
    let mut client_impl = String::new();
    let mut backend_reads = String::new();
    let mut backend_writes = String::new();
    let mut client_backend_writes = String::new();
    let mut backend_read_impl = String::new();
    let mut backend_write_impl = String::new();
    let mut client_backend_write_impl = String::new();
    // Each backend read's name, signature, argument names and typed refusal, for the goal view.
    let mut backend_read_list: Vec<(&str, String, String, &str)> = Vec::new();
    // Each draw context's trait methods and their forwarding by its game struct.
    let mut drawing: Vec<(String, String)> = contexts.iter().map(|_| Default::default()).collect();
    let mut handles = std::collections::BTreeMap::<&str, String>::new();
    for helper in &catalog.helpers {
        let (name, side, target) = (helper.name.as_str(), helper.side, helper.target.as_str());
        let method = helper_target(&game, name, target)?;
        if let Some((index, declaration, implementation)) =
            draw_method(method, name, contexts, &types.exports)?
        {
            if side != Side::Client {
                return Err(format!("{name}: drawing is client-side"));
            }
            drawing[index].0.push_str(&declaration);
            drawing[index].1.push_str(&implementation);
            continue;
        }
        let receiver = helper.receiver.as_deref();
        let is_backend = helper.backend;
        let trait_name = if receiver.is_some() || is_backend {
            if matches!(method.sig.inputs.first(), Some(FnArg::Receiver(r)) if r.mutability.is_some())
            {
                if side == Side::Client {
                    "ClientBackendMut"
                } else {
                    "BackendMut"
                }
            } else {
                "Backend"
            }
        } else {
            match side {
                Side::Both => "Host",
                Side::Server => "ServerHost",
                Side::Client => "ClientHost",
            }
        };
        let (declaration, implementation) = host_method(method, name, trait_name, &types.exports)?;
        if let Some(client_target) = &helper.client_target {
            let client = helper_target(&game, name, client_target)?;
            let (_, client_implementation) = host_method(client, name, trait_name, &types.exports)?;
            matching_helper_signatures(&implementation, &client_implementation, name)?;
        }
        let side_doc = match side {
            Side::Server => "/// Side: server.\n",
            Side::Client => "/// Side: client.\n",
            Side::Both => "/// Side: both.\n",
        };
        if receiver.is_some() || is_backend {
            let mutable = matches!(method.sig.inputs.first(), Some(FnArg::Receiver(r)) if r.mutability.is_some());
            if !mutable {
                let mut sig = method.sig.clone();
                sig.ident = syn::parse_str(name).map_err(|e| format!("{name}: {e}"))?;
                let args = sig
                    .inputs
                    .iter()
                    .skip(1)
                    .map(|arg| match arg {
                        FnArg::Typed(arg) => match &*arg.pat {
                            Pat::Ident(pat) => Ok(pat.ident.to_string()),
                            _ => Err(format!("{name}: unsupported argument")),
                        },
                        FnArg::Receiver(_) => Err(format!("{name}: unexpected receiver")),
                    })
                    .collect::<Result<Vec<_>, String>>()?
                    .join(", ");
                let refusal = match &sig.output {
                    syn::ReturnType::Default => {
                        return Err(format!("{name}: a backend read returns a value"));
                    }
                    syn::ReturnType::Type(_, ty) => match &**ty {
                        syn::Type::Path(path)
                            if path
                                .path
                                .segments
                                .last()
                                .is_some_and(|seg| seg.ident == "Option") =>
                        {
                            "None"
                        }
                        syn::Type::Path(path)
                            if path
                                .path
                                .segments
                                .last()
                                .is_some_and(|seg| seg.ident == "bool") =>
                        {
                            "false"
                        }
                        syn::Type::Path(path)
                            if path.path.segments.last().is_some_and(|seg| {
                                matches!(
                                    seg.ident.to_string().as_str(),
                                    "u32" | "i32" | "u16" | "u8" | "usize"
                                )
                            }) =>
                        {
                            "0"
                        }
                        _ => {
                            return Err(format!(
                                "{name}: the goal view cannot refuse its return type"
                            ));
                        }
                    },
                };
                backend_read_list.push((name, sig.to_token_stream().to_string(), args, refusal));
            }
            let (trait_methods, impl_methods) = if mutable && side == Side::Client {
                (&mut client_backend_writes, &mut client_backend_write_impl)
            } else if mutable {
                (&mut backend_writes, &mut backend_write_impl)
            } else {
                (&mut backend_reads, &mut backend_read_impl)
            };
            trait_methods.push_str(side_doc);
            trait_methods.push_str(&declaration);
            impl_methods.push_str(&implementation);
            if let Some(receiver) = receiver {
                let (handle, code) = handle_method(method, name, receiver, side, &types.handles)?;
                let methods = handles.entry(handle).or_default();
                methods.push_str(side_doc);
                methods.push_str(&code);
            }
        } else {
            let (traits, implementation_out) = match side {
                Side::Both => (&mut host_traits, &mut host_impl),
                Side::Server => (&mut server_traits, &mut server_impl),
                Side::Client => (&mut client_traits, &mut client_impl),
            };
            traits.push_str(side_doc);
            traits.push_str(&declaration);
            implementation_out.push_str(&implementation);
        }
    }
    let handles = handles
        .iter()
        .map(|(handle, methods)| format!("impl {handle} {{\n{methods}}}\n"))
        .collect::<String>();
    struct StdTypes {
        path_buf: bool,
        arc: bool,
    }
    impl<'ast> Visit<'ast> for StdTypes {
        fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
            if ty.path.segments.len() == 1 {
                self.path_buf |= ty.path.segments[0].ident == "PathBuf";
                self.arc |= ty.path.segments[0].ident == "Arc";
            }
            syn::visit::visit_type_path(self, ty);
        }
    }
    let host_trait: syn::ItemTrait = syn::parse_str(&format!(
        "trait Host {{ {host_traits} {server_traits} {client_traits} {backend_reads} {backend_writes} {client_backend_writes} }}"
    ))
    .map_err(|e| format!("generated Host: {e}"))?;
    let mut std_types = StdTypes {
        path_buf: false,
        arc: false,
    };
    std_types.visit_item_trait(&host_trait);
    let mut imports = String::new();
    if std_types.path_buf {
        imports.push_str("use std::path::PathBuf;\n");
    }
    if std_types.arc {
        imports.push_str("use std::sync::Arc;\n");
    }
    let side_enum = |name: &str, server: &str, client: &str| {
        format!(
            "/// The logical side of a hook's host.\npub enum {name}<'a> {{\n/// The hook fired on the server.\nServer({server}),\n/// The hook fired on the client.\nClient({client}) }}\n"
        )
    };
    let sides = format!(
        "{}{}",
        side_enum(
            "HostSide",
            "&'a mut dyn ServerHost",
            "&'a mut dyn ClientHost"
        ),
        side_enum("LevelSide", "&'a dyn LevelHost", "&'a dyn LevelHost")
    );
    let mut draw_traits = String::new();
    let mut draw_impls = String::new();
    for (context, (declarations, implementations)) in contexts.iter().zip(&drawing) {
        let (doc, trait_name, game_struct, surface) = (
            &context.doc,
            &context.trait_name,
            context.game_struct(),
            &context.surface,
        );
        draw_traits.push_str(&format!(
            "/// {doc}\n/// Each method's default panics with its name, so a test double implements only what its test calls.\npub trait {trait_name} {{\n{declarations}}}\n"
        ));
        draw_impls.push_str(&format!(
            "/// [`{trait_name}`] on the game's `{surface}`.\npub(crate) struct {game_struct}<'a, 'g> {{\npub(crate) game: &'a super::Game<'g>,\npub(crate) surface: &'a mut {surface},\n}}\nimpl {trait_name} for {game_struct}<'_, '_> {{\n{implementations}}}\n"
        ));
    }
    let host = format!(
        "// @generated by cargo run -p xtask -- mod-api; do not edit.\nuse super::*;\n{imports}/// Services supplied by the game to trait mods.\n/// Each method's default panics with its name, so a test double implements only what its test calls; the game implements every one.\n/// The backend accessors are the exception: they default to None, so a double without a backend answers every handle's typed refusal.\npub trait Host {{\n#[doc(hidden)] fn backend(&self) -> Option<BackendRef<'_>> {{ None }}\n#[doc(hidden)] fn capability_providers(&self) -> &'static dyn CapabilityProviders {{ &InstalledProviders }}\n{host_traits}}}\n/// What only the server's hooks are given: writes to the world.\npub trait ServerHost: Host {{\n#[doc(hidden)] fn backend_mut(&mut self) -> Option<BackendMutRef<'_>> {{ None }}\n{server_traits}}}\n/// What only the client's hooks are given: the client's own services.\npub trait ClientHost: Host {{\n#[doc(hidden)] fn client_backend_mut(&mut self) -> Option<ClientBackendMutRef<'_>> {{ None }}\n{client_traits}}}\n{sides}#[doc(hidden)] pub trait Backend {{\n{backend_reads}}}\n#[doc(hidden)] pub trait BackendMut {{\n{backend_writes}}}\n#[doc(hidden)] pub trait ClientBackendMut {{\n{client_backend_writes}}}\n{handles}{draw_traits}"
    );
    let implementation = format!(
        "// @generated by cargo run -p xtask -- mod-api; do not edit.\nuse mod_api::*;\nuse mod_api::__private::game::*;\n{imports}impl Host for super::Game<'_> {{\nfn backend(&self) -> Option<BackendRef<'_>> {{ Some(BackendRef::of(self)) }}\n{host_impl}}}\nimpl ServerHost for super::Game<'_> {{\nfn backend_mut(&mut self) -> Option<BackendMutRef<'_>> {{ Some(BackendMutRef::of(self)) }}\n{server_impl}}}\nimpl ClientHost for super::Game<'_> {{\nfn client_backend_mut(&mut self) -> Option<ClientBackendMutRef<'_>> {{ Some(ClientBackendMutRef::of(self)) }}\n{client_impl}}}\nimpl Backend for super::Game<'_> {{\n{backend_read_impl}}}\nimpl BackendMut for super::Game<'_> {{\n{backend_write_impl}}}\nimpl ClientBackendMut for super::Game<'_> {{\n{client_backend_write_impl}}}\n{draw_impls}"
    );
    let goal_view = goal_view(&backend_read_list)?;
    Ok((host, implementation, goal_view))
}

/// The goal view's `Backend` impl (#1077): the reads a mob's goal tick answers
/// (`GOAL_VIEW_READS`) forward to the engine context's `r_*` methods; every
/// other backend read is refused its typed refusal, as a host without a
/// backend refuses it. The engine includes this beside its `GoalCx`.
fn goal_view(reads: &[(&str, String, String, &str)]) -> Result<String, String> {
    let mut methods = String::new();
    for (name, sig, args, refusal) in reads {
        let body = if GOAL_VIEW_READS.contains(name) {
            format!("self.r_{name}({args})")
        } else if args.is_empty() {
            (*refusal).to_string()
        } else {
            // The refusal touches none of its arguments.
            if args.contains(',') {
                format!("let _ = ({args}); {refusal}")
            } else {
                format!("let _ = {args}; {refusal}")
            }
        };
        methods.push_str(&format!("{sig} {{ {body} }}\n"));
    }
    Ok(format!(
        "// @generated by cargo run -p xtask -- mod-api; do not edit.\nuse mod_api::*;\n/// The mob tick's read view (#1077), as the game generates it: the snapshot-backed\n/// reads forward to the engine context's own methods, every other backend read answers\n/// its typed refusal.\nimpl Backend for super::GoalCx<'_, '_> {{\n{methods}}}"
    ))
}

/// A draw helper's method on its context's trait, when one of `method`'s inputs is a draw
/// surface: the context's index, the trait's declaration, and the game struct's forwarding.
/// The trait method borrows itself as the helper borrows the surface; the helper borrows
/// `Game` shared, as the struct holds it.
fn draw_method(
    method: &syn::ItemFn,
    name: &str,
    contexts: &[DrawContext],
    exports: &HashSet<String>,
) -> Result<Option<(usize, String, String)>, String> {
    let Some((position, context, mutable)) =
        method
            .sig
            .inputs
            .iter()
            .enumerate()
            .find_map(|(position, arg)| match arg {
                FnArg::Typed(arg) => DrawContext::of(contexts, &arg.ty)
                    .map(|(context, mutable)| (position, context, mutable)),
                FnArg::Receiver(_) => None,
            })
    else {
        return Ok(None);
    };
    if !matches!(method.sig.inputs.first(), Some(FnArg::Receiver(r)) if r.reference.is_some() && r.mutability.is_none())
    {
        return Err(format!("{name}: a draw helper borrows Game shared (&self)"));
    }
    let index = contexts
        .iter()
        .position(|other| std::ptr::eq(other, context))
        .expect("context found");
    let short = name.strip_prefix(context.prefix.as_str()).ok_or_else(|| {
        format!(
            "{name}: a {} helper's name starts with {}",
            context.trait_name, context.prefix
        )
    })?;
    let mut stripped = method.clone();
    stripped.sig.inputs = stripped
        .sig
        .inputs
        .into_iter()
        .enumerate()
        .filter(|(at, _)| *at != position)
        .map(|(at, arg)| match at {
            0 if mutable => syn::parse_quote!(&mut self),
            _ => arg,
        })
        .collect();
    let (declaration, _) = host_method(&stripped, short, &context.trait_name, exports)?;
    let sig = syn::parse_str::<syn::TraitItemFn>(&declaration)
        .map_err(|e| format!("{name}: {e}"))?
        .sig;
    let surface = if mutable {
        "&mut *self.surface"
    } else {
        "&*self.surface"
    };
    let args = method
        .sig
        .inputs
        .iter()
        .enumerate()
        .skip(1)
        .map(|(at, arg)| match arg {
            _ if at == position => Ok(surface.to_owned()),
            FnArg::Typed(arg) => match &*arg.pat {
                Pat::Ident(pat) => Ok(pat.ident.to_string()),
                _ => Err(format!("{name}: unsupported argument")),
            },
            FnArg::Receiver(_) => Err(format!("{name}: unexpected receiver")),
        })
        .collect::<Result<Vec<_>, String>>()?;
    let implementation = format!(
        "{} {{ self.game.{}({}) }}\n",
        sig.to_token_stream(),
        method.sig.ident,
        args.join(", ")
    );
    Ok(Some((index, declaration, implementation)))
}
