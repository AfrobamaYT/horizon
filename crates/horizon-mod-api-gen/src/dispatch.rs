use super::*;

pub(super) struct HookSources {
    pub traits: String,
    pub dispatch: String,
    pub render_traits: String,
    pub render_dispatch: String,
    pub part_traits: BTreeMap<&'static str, String>,
    pub fills: Vec<Fill>,
    pub hook_ids: String,
    pub hook_constants: String,
    pub hook_count: usize,
    pub render_hook_ids: String,
    pub render_hook_constants: String,
    pub render_hook_count: usize,
}

pub(super) fn emit(
    root: &Path,
    hooks: &[CatalogHook],
    methods: &[syn::ItemFn],
    contexts: &[DrawContext],
) -> Result<HookSources, String> {
    let mut hook_ids = String::new();
    let mut hook_constants = String::new();
    let mut hook_count = 0;
    let mut render_hook_ids = String::new();
    let mut render_hook_constants = String::new();
    let mut render_hook_count = 0;
    let mut traits = String::new();
    let mut dispatch = String::new();
    let mut render_traits = String::new();
    let mut render_dispatch = String::new();
    let mut part_traits: BTreeMap<&str, String> = BTreeMap::new();
    let mut fills = Vec::new();
    for hook in hooks {
        let name = hook["name"].as_str().ok_or("hook name missing")?;
        let api = hook
            .get("mod_api")
            .ok_or_else(|| format!("hook {name} has no mod_api entry: add one"))?;
        let native = format!("native_{name}");
        let target = hook["target"].as_str().ok_or("target missing")?;
        let is_render = target.starts_with("mod_api::render::");
        let external = free_function_source(target);
        let in_game = external.is_none();
        let (method, call) = if let Some((krate, path)) = external {
            let Some((module, function)) = path.rsplit_once("::") else {
                return Err(format!("wrong target for {name}"));
            };
            if function != native {
                return Err(format!("wrong target for {name}"));
            }
            let path = root
                .join("crates")
                .join(krate)
                .join("src")
                .join(format!("{}.rs", module.replace("::", "/")));
            let source =
                fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let file = syn::parse_file(&source).map_err(|e| e.to_string())?;
            let functions: Vec<_> = file
                .items
                .into_iter()
                .filter_map(|item| match item {
                    Item::Fn(function) => Some(function),
                    _ => None,
                })
                .collect();
            (
                one_function(functions.iter(), &native)?.clone(),
                // A hook in the mod API crate calls its dispatch directly; one below it calls
                // the callback the game installs.
                if krate == "horizon-mod-api" {
                    format!("dispatch_{name}")
                } else {
                    name.to_owned()
                },
            )
        } else {
            if !target.starts_with("horizon_client::game::native_hooks::")
                || !target.ends_with(&format!("::{native}"))
            {
                return Err(format!("wrong target for {name}"));
            }
            (
                one_function(methods.iter(), &native)?.clone(),
                format!("dispatch_{name}"),
            )
        };
        let fill = needs_fill(&method, &call, in_game)?;
        let sig = &method.sig;
        let mut args = Vec::new();
        let mut receiver_mut = None;
        for arg in &sig.inputs {
            match arg {
                FnArg::Receiver(r) => {
                    receiver_mut = Some(r.mutability.is_some());
                }
                FnArg::Typed(arg) => {
                    let Pat::Ident(pat) = &*arg.pat else {
                        return Err(format!("unsupported argument in {name}"));
                    };
                    args.push((pat.ident.to_string(), &*arg.ty));
                }
            }
        }
        let is_level = host_param::is_level(hook);
        let ticks = is_level
            && args.iter().any(|(name, ty)| {
                name == "ticks" && ty.to_token_stream().to_string() == "& mut Ticks"
            });
        host_param::validate(hook, name, is_level, receiver_mut, ticks)?;
        let context = host_param::context_param(hook, &mut args)?;
        if ticks {
            args.retain(|(name, _)| name != "ticks");
        }
        let mut used: HashSet<_> = args.iter().map(|(name, _)| name.clone()).collect();
        let mod_item = fresh_name("mod_hook", &mut used);
        let part_list = fresh_name("parts", &mut used);
        let part_item = fresh_name("part", &mut used);
        let part_table = fresh_name("part_table", &mut used);
        let replacement = fresh_name("replacement", &mut used);
        let contract = api["contract"].as_str().ok_or("contract missing")?;
        let out = api.get("out").and_then(|v| v.as_str());
        // A replace hook that names which mod answered: its output holds `Answered<T>`.
        let answered = api.get("answered_by").is_some_and(|v| v == "mod");
        if answered && (contract != "replace" || api.get("scope").is_some()) {
            return Err(format!("{name}: answered_by needs a global replace hook"));
        }
        if !matches!(
            (contract, out),
            ("cancel" | "replace" | "collect", Some(_)) | ("listen", None)
        ) {
            return Err(format!("invalid contract for {name}"));
        }
        let output = if let Some(out) = out {
            let index = args
                .iter()
                .position(|(n, _)| n == out)
                .ok_or_else(|| format!("missing output {out} in {name}"))?;
            Some(args.remove(index))
        } else {
            None
        };
        let result = if let Some((_, ty)) = &output {
            let Type::Reference(reference) = ty else {
                return Err(format!("output must be a mutable reference in {name}"));
            };
            if reference.mutability.is_none() {
                return Err(format!("output must be a mutable reference in {name}"));
            }
            match contract {
                "cancel" if reference.elem.to_token_stream().to_string() == "bool" => {
                    " -> Flow".to_owned()
                }
                "replace" if answered => {
                    let answer = answered_type(&reference.elem)
                        .ok_or_else(|| format!("{name}: output must be Option<Answered<T>>"))?;
                    format!(" -> Option<{}>", answer.to_token_stream())
                }
                "replace" => format!(" -> {}", reference.elem.to_token_stream()),
                "collect" => String::new(),
                _ => return Err(format!("unexpected output type for {name}")),
            }
        } else {
            String::new()
        };
        let host = context
            .or_else(|| host_param::host_param(hook, receiver_mut, ticks))
            .map(|param| {
                (
                    fresh_name(
                        if hook.context.is_some() {
                            "context"
                        } else if is_level {
                            "level"
                        } else {
                            "host"
                        },
                        &mut used,
                    ),
                    param,
                )
            });
        // A scoped hook's parts: the trait its method goes on, and the lookup
        // of the parts attached to the thing its `key` argument names.
        let scoped = match (api.get("scope"), api.get("key")) {
            (None, None) => None,
            (Some(scope), Some(key)) => {
                let key = key.as_str().ok_or("key must be a string")?;
                let (part_trait, table) = match scope.as_str() {
                    Some("block") => ("BlockPart", "block"),
                    Some("block_entity") => ("BlockEntityPart", "block_entity"),
                    Some("item") => ("ItemPart", "item"),
                    Some("menu") => ("MenuPart", "menu"),
                    _ => return Err(format!("unknown scope for {name}")),
                };
                if is_render {
                    return Err(format!("render hook {name} cannot be scoped"));
                }
                let (_, ty) = args
                    .iter()
                    .find(|(arg, _)| arg == key)
                    .ok_or_else(|| format!("key {key} is not an argument of {name}"))?;
                let lookup = match ty.to_token_stream().to_string().as_str() {
                    "BlockStateId" if table == "block" => format!("block_state({key})"),
                    "BlockStateId" => {
                        let Some((level, _)) = host.as_ref().filter(|_| is_level) else {
                            return Err(format!("{name} has no level to name {key}'s block"));
                        };
                        if hook.side == HelperSide::Both {
                            format!(
                                "{table}(|| match &{level} {{ LevelSide::Server({level}) | LevelSide::Client({level}) => {level}.state_block({key}), }})"
                            )
                        } else {
                            format!("{table}(|| {level}.state_block({key}))")
                        }
                    }
                    "& ItemStack" => format!("{table}(|| Some(&*{key}.item))"),
                    "& str" => format!("{table}(|| Some({key}))"),
                    // An entity's type id need not be its block's: its own table.
                    "& ModBlockEntity" | "& mut ModBlockEntity" => {
                        format!("block_entity_kind(|| Some({key}.id()))")
                    }
                    other => return Err(format!("key {key} of {name} has type {other}")),
                };
                Some((part_trait, lookup))
            }
            _ => return Err(format!("{name} needs both scope and key, or neither")),
        };
        let event = fresh_name("event", &mut used);
        let event_type = format!("{}Event", hook_id(name));
        let hook_id = event_type.strip_suffix("Event").unwrap();
        if is_render {
            render_hook_ids.push_str(&format!("{hook_id},\n"));
            render_hook_constants.push_str(&format!(
                "pub const {name}: Option<super::RenderHookId> = Some(super::RenderHookId::{hook_id});\n"
            ));
            render_hook_count += 1;
        } else {
            hook_ids.push_str(&format!("{hook_id},\n"));
            hook_constants.push_str(&format!(
                "pub const {name}: Option<super::HookId> = Some(super::HookId::{hook_id});\n"
            ));
            hook_count += 1;
        }
        let mut trait_params = vec!["&self".to_owned()];
        let mut dispatch_params = Vec::new();
        let mut call_args = Vec::new();
        if let Some((name, param)) = &host {
            trait_params.push(format!("{name}: {}", param.ty));
            let by_side = param.ty == "HostSide" || param.ty == "LevelSide";
            dispatch_params.push(format!(
                "{}{name}: {}",
                if by_side { "mut " } else { "" },
                param.ty
            ));
            call_args.push(if by_side {
                format!("{name}.reborrow()")
            } else if param.ty.starts_with("&mut ") {
                format!("&mut *{name}")
            } else {
                name.clone()
            });
        }

        let mut lifetimes = EventLifetimes::default();
        let mut fields = String::new();
        let mut inits = Vec::new();
        let mut write_back = String::new();
        let collected = output.as_ref().filter(|_| contract == "collect");
        // A draw surface reaches mods as its context's trait object, which the native call
        // builds from `Game` and the surface.
        let mut surface_calls = std::collections::HashMap::new();
        for (name, ty) in args.iter().chain(collected) {
            let drawn = DrawContext::of(contexts, ty);
            if let Some((context, mutable)) = drawn {
                if !mutable || receiver_mut != Some(false) {
                    return Err(format!(
                        "{}: a drawing hook takes &self and its surface as &mut",
                        hook["name"].as_str().unwrap_or_default()
                    ));
                }
                surface_calls.insert(
                    name.as_str(),
                    format!(
                        "&mut super::mod_api_host::generated::{} {{ game: self, surface: {name} }}",
                        context.game_struct()
                    ),
                );
            }
            let drawn: Option<Type> = drawn.map(|(context, _)| {
                syn::parse_str(&format!("&mut dyn {}", context.trait_name)).expect("a trait name")
            });
            let ty = drawn.as_ref().unwrap_or(ty);
            dispatch_params.push(format!("{name}: {}", ty.to_token_stream()));
            let mutable_entity = ty.to_token_stream().to_string() == "& mut ModBlockEntity";
            let field_ty = if mutable_entity {
                syn::parse_quote!(ModBlockEntityView<'_>)
            } else {
                ty.clone()
            };
            let (field, scalar) = lifetimes.field(&field_ty);
            let visibility = if event_type == "ApplyEffectTickEvent" && name == "changes" {
                "pub(super)"
            } else {
                "pub"
            };
            fields.push_str(&format!(
                "{visibility} {name}: {},\n",
                field.to_token_stream()
            ));
            if scalar {
                inits.push(format!("{name}: *{name}"));
                write_back.push_str(&format!("*{name} = {event}.{name};"));
            } else if mutable_entity {
                inits.push(format!("{name}: ModBlockEntityView::new({name})"));
            } else {
                inits.push(name.clone());
            }
        }
        if let Some((name, ty)) = output.as_ref().filter(|_| contract != "collect") {
            dispatch_params.push(format!("{name}: {}", ty.to_token_stream()));
        }
        let (generics, elided) = match (lifetimes.a, lifetimes.b) {
            (_, true) => ("<'a, 'b>", "<'_, '_>"),
            (true, false) => ("<'a>", "<'_>"),
            (false, false) => ("", ""),
        };
        trait_params.push(format!("{event}: &mut {event_type}{elided}"));
        call_args.push(format!("&mut {event}"));
        let unused = trait_params[1..]
            .iter()
            .map(|param| format!("let _ = {};", param.split(':').next().unwrap()))
            .collect::<String>();
        let default = match contract {
            "cancel" => "Flow::Continue",
            "replace" => "None",
            _ => "",
        };
        if fill {
            // The dispatch's parameters in order: the host, the arguments,
            // and the output last.
            let host = host.as_ref().map(|(_, param)| param.call.to_owned());
            let passed = host
                .into_iter()
                .chain(args.iter().chain(output.as_ref()).map(|(n, _)| {
                    surface_calls
                        .get(n.as_str())
                        .cloned()
                        .unwrap_or_else(|| n.clone())
                }))
                .collect::<Vec<_>>()
                .join(", ");
            fills.push(Fill {
                native: native.clone(),
                call: format!("mod_api::{call}({passed});"),
            });
        }
        let invoke = call_args.join(", ");
        let trait_params = trait_params.join(", ");
        let dispatch_params = dispatch_params.join(", ");
        let trait_name = if is_render { "RenderHooks" } else { "Hooks" };
        let traits = if is_render {
            &mut render_traits
        } else {
            &mut traits
        };
        let dispatch = if is_render {
            &mut render_dispatch
        } else {
            &mut dispatch
        };
        let mut method_source = String::new();
        for line in hook["doc"].as_str().ok_or("doc missing")?.trim().lines() {
            method_source.push_str(&format!(
                "///{}{line}\n",
                if line.is_empty() { "" } else { " " }
            ));
        }
        let side = match hook.side {
            HelperSide::Server => "server",
            HelperSide::Client => "client",
            HelperSide::Both => "both",
        };
        method_source.push_str(&format!("/// Side: {side}.\n"));
        // A scoped hook is its part's method alone: a global moment beside it is its own hook.
        let callee = if scoped.is_some() { "part" } else { "mod" };
        match (contract, out) {
            ("cancel", Some(out)) => method_source.push_str(&format!(
                "///\n/// Return `Flow::Cancel` to take it: this sets `{out}`, and later {callee}s are skipped.\n"
            )),
            ("replace", Some(out)) => method_source.push_str(&format!(
                "///\n/// Return `Some` to set `{out}`; the first {callee} returning `Some` wins.\n"
            )),
            ("collect", Some(out)) => method_source.push_str(&format!(
                "///\n/// Every {callee} gets `{event}.{out}` in order and may add, change or remove {out}.\n"
            )),
            _ => {}
        }
        method_source.push_str(&format!(
            "fn {name}({trait_params}){result} {{ {unused} {default} }}\n"
        ));
        match &scoped {
            Some((part_trait, _)) => part_traits
                .entry(*part_trait)
                .or_default()
                .push_str(&method_source),
            None => traits.push_str(&method_source),
        }
        let trait_name = scoped
            .as_ref()
            .map_or(trait_name, |(part_trait, _)| part_trait);
        dispatch.push_str(&format!(
            "/// What [`{trait_name}::{name}`] is told.\npub struct {event_type}{generics} {{\n{fields}}}\n"
        ));
        let build = format!("let mut {event} = {event_type} {{ {} }};", inits.join(", "));
        // A trait mod's call is marked with the mod for a panic to name; render hooks are called bare.
        let each = if is_render {
            format!("super::entered_render(RenderHookId::{hook_id})")
        } else if answered {
            format!("super::entered_with_id(HookId::{hook_id}, \"{name}\")")
        } else {
            format!("super::entered(HookId::{hook_id}, \"{name}\")")
        };
        // One callee's call: a mod's, or a scoped hook's part's.
        let call = |callee: &str, exit: &str| match contract {
            "cancel" => format!(
                "if matches!({callee}.{name}({invoke}), Flow::Cancel) {{ *{o} = true; {exit}; }}",
                o = out.unwrap()
            ),
            "replace" if answered => format!(
                "if let Some({replacement}) = {callee}.{name}({invoke}) {{ *{o} = Some(Answered {{ answer: {replacement}, mod_id }}); {exit}; }}",
                o = out.unwrap()
            ),
            "replace" => format!(
                "if let Some({replacement}) = {callee}.{name}({invoke}) {{ *{o} = Some({replacement}); {exit}; }}",
                o = out.unwrap()
            ),
            _ => format!("{callee}.{name}({invoke});"),
        };
        let binding = if answered {
            format!("(mod_id, {mod_item})")
        } else {
            mod_item.clone()
        };
        let (lookup, calls) = match &scoped {
            None => (
                String::new(),
                format!("for {binding} in {each} {{ {} }}", call(&mod_item, "break")),
            ),
            Some((_, lookup)) => (
                // The parts are looked up first: the key may be moved into the event.
                format!(
                    "\nif !super::has_part_hook(HookId::{hook_id}) {{ return; }} let {part_table} = super::parts(\"{name}\"); let {part_list} = {part_table}.for_hook(HookId::{hook_id}).map_or(&[][..], |table| table.{lookup}); if {part_list}.is_empty() {{ return; }}"
                ),
                format!(
                    "for {part_item} in super::entered_parts({part_list}, \"{name}\") {{ {} }}",
                    call(&part_item, "break"),
                ),
            ),
        };
        let body = match contract {
            "cancel" => format!(
                "if *{o} {{ return; }} {lookup} {build} {calls} {write_back}",
                o = out.unwrap()
            ),
            "replace" => format!(
                "if {o}.is_some() {{ return; }} {lookup} {build} {calls} {write_back}",
                o = out.unwrap()
            ),
            _ => format!("{lookup} {build} {calls} {write_back}"),
        };
        let early_return = if scoped.is_some() {
            String::new()
        } else if is_render {
            format!("if !super::render_listened(RenderHookId::{hook_id}) {{ return; }}")
        } else {
            format!("if !super::listened(HookId::{hook_id}) {{ return; }}")
        };
        dispatch.push_str(&format!(
            "pub fn dispatch_{name}({dispatch_params}) {{ {early_return}\n{body} }}\n"
        ));
    }
    Ok(HookSources {
        traits,
        dispatch,
        render_traits,
        render_dispatch,
        part_traits,
        fills,
        hook_ids,
        hook_constants,
        hook_count,
        render_hook_ids,
        render_hook_constants,
        render_hook_count,
    })
}

/// `T` of an `Option<Answered<T>>` output, if that is its type.
fn answered_type(ty: &Type) -> Option<&Type> {
    generic_of(generic_of(ty, "Option")?, "Answered")
}

/// `T` of a `name<T>` type.
fn generic_of<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last().filter(|s| s.ident == name)?;
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        syn::GenericArgument::Type(ty) => Some(ty),
        _ => None,
    }
}
