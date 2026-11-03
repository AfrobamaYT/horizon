use super::{CatalogHook, HelperSide, HookContext};
use syn::Type;

/// The hook's host parameter type and the expression passed by its native method.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct HostParam {
    pub ty: String,
    pub call: &'static str,
}

/// Validate catalog access restrictions against the hook's receiver.
pub(super) fn validate(
    hook: &CatalogHook,
    name: &str,
    is_level: bool,
    mutable: Option<bool>,
    ticks: bool,
) -> Result<(), String> {
    if hook.context.is_some() {
        let owns_host = mutable.is_some() || is_level || ticks;
        let host_marker = hook.get("host").is_some();
        let scoped = hook["mod_api"].get("scope").is_some();
        if owns_host || host_marker || scoped {
            return Err(format!(
                "{name}: a phase context cannot be combined with a game/level host or Parts scope"
            ));
        }
    }
    if let Some(marker) = hook.get("host") {
        if is_level || hook.side == HelperSide::Client {
            return Err(format!("{name}: this entry takes no host marker"));
        }
        if marker.as_str() != Some("read") {
            return Err(format!("{name}: invalid host marker"));
        }
    }
    if !is_level
        && hook.side == HelperSide::Both
        && mutable == Some(false)
        && hook["host"].as_str() != Some("read")
    {
        return Err(format!(
            "{name}: a both-side hook with a &self target takes host \"read\""
        ));
    }
    if is_level && hook.side == HelperSide::Both && (mutable == Some(true) || ticks) {
        return Err(format!(
            "{name}: a both-side level hook takes the read kind until a client level seam exists"
        ));
    }
    Ok(())
}

/// Remove the native phase input from the event, selecting its public context.
pub(super) fn context_param(
    hook: &CatalogHook,
    args: &mut Vec<(String, &Type)>,
) -> Result<Option<HostParam>, String> {
    let Some(kind) = hook.context else {
        return Ok(None);
    };
    let name = hook["name"].as_str().ok_or("hook name missing")?;
    let expected = match kind {
        HookContext::Registration => "RegistrationContext",
        HookContext::DataLoad => "LoadContext",
    };
    let valid = args.first().is_some_and(|(name, ty)| {
        name == "context"
            && matches!(ty, Type::Reference(reference)
            if reference.mutability.is_none() && matches!(&*reference.elem, Type::Path(path)
                if path.path.segments.last().is_some_and(|segment| segment.ident == expected)))
    });
    if !valid {
        return Err(format!(
            "{name}: phase context requires first input context: &{expected}<'_>"
        ));
    }
    args.remove(0);
    Ok(Some(HostParam {
        ty: match kind {
            HookContext::Registration => "&RegistrationContext<'_>",
            HookContext::DataLoad => "&DataLoadContext<'_>",
        }
        .into(),
        call: "context",
    }))
}

/// Select the host once for both generated signatures and the native call.
pub(super) fn host_param(
    hook: &CatalogHook,
    mutable: Option<bool>,
    ticks: bool,
) -> Option<HostParam> {
    let is_level = hook["target"]
        .as_str()
        .is_some_and(|target| target.contains("::GameLevel<"));
    mutable.map(|mutable| {
        if !is_level {
            let (ty, call) = match hook.side {
                HelperSide::Server | HelperSide::Both if hook["host"].as_str() == Some("read") => {
                    ("&dyn Host", "&*self")
                }
                HelperSide::Server => ("&mut dyn ServerHost", "self"),
                HelperSide::Client if mutable => ("&mut dyn ClientHost", "self"),
                HelperSide::Client => ("&dyn ClientHost", "&*self"),
                HelperSide::Both => ("HostSide", "HostSide::of(self)"),
            };
            return HostParam {
                ty: ty.into(),
                call,
            };
        }
        if hook.side == HelperSide::Both {
            return HostParam {
                ty: "LevelSide".into(),
                call: "super::mod_api_host::level_side(self)",
            };
        }
        let kind = if is_level {
            match (mutable, ticks) {
                (true, true) => "LevelWriteHost",
                (false, true) => "LevelTicksHost",
                (true, false) => "LevelRandomHost",
                (false, false) => "LevelHost",
            }
        } else {
            "Host"
        };
        let mutable = if mutable || (is_level && ticks) {
            "mut "
        } else {
            ""
        };
        HostParam {
            ty: format!("&{mutable}dyn {kind}"),
            call: if ticks {
                "&mut super::mod_api_host::LevelTicks { level: self, ticks }"
            } else {
                "self"
            },
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_context_is_not_an_event_field_or_game_host() {
        for (kind, input, public) in [
            (
                HookContext::Registration,
                "&RegistrationContext<'_>",
                "&RegistrationContext<'_>",
            ),
            (
                HookContext::DataLoad,
                "&LoadContext<'_>",
                "&DataLoadContext<'_>",
            ),
        ] {
            let hook = CatalogHook {
                entry: serde_json::json!({"name": "phase", "side": "both"}),
                side: HelperSide::Both,
                context: Some(kind),
            };
            let ty = syn::parse_str(input).unwrap();
            let output = syn::parse_quote!(&mut Vec<String>);
            let mut args = vec![("context".into(), &ty), ("values".into(), &output)];
            assert_eq!(
                context_param(&hook, &mut args).unwrap(),
                Some(HostParam {
                    ty: public.into(),
                    call: "context",
                })
            );
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].0, "values");
            assert!(validate(&hook, "phase", false, None, false).is_ok());
            assert!(validate(&hook, "phase", false, Some(true), false).is_err());
            assert!(context_param(&hook, &mut args).is_err());
            let mutable = syn::parse_str(&input.replacen('&', "&mut ", 1)).unwrap();
            assert!(context_param(&hook, &mut vec![("context".into(), &mutable)]).is_err());
        }
    }
}
