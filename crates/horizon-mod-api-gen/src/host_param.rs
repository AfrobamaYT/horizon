use super::{CatalogHook, HelperSide};

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
    if let Some(marker) = hook.get("host") {
        if is_level || hook.side == HelperSide::Client {
            return Err(format!("{name}: this entry takes no host marker"));
        }
        if marker.as_str() != Some("read") {
            return Err(format!("{name}: invalid host marker"));
        }
    }
    if is_level && hook.side == HelperSide::Both && (mutable == Some(true) || ticks) {
        return Err(format!(
            "{name}: a both-side level hook takes the read kind until a client level seam exists"
        ));
    }
    Ok(())
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
