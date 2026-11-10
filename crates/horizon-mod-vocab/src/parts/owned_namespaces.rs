use super::*;

/// The package inventory carries the same namespace set as content files and
/// the package loader's collision fence. A package always owns its own ID.
pub(crate) fn owns_namespace(owner: &str, namespace: &str) -> bool {
    owner == namespace
        || crate::LoadedMods::new()
            .get(owner)
            .is_some_and(|package| package.namespaces().any(|owned| owned == namespace))
}

pub(crate) fn owned_declaration_id(owner: &str, name: &str) -> Result<String, String> {
    let id = if name.contains(':') {
        name.to_owned()
    } else {
        format!("{owner}:{name}")
    };
    let parsed = crate::Identifier::parse_namespaced(&id)
        .ok_or_else(|| format!("mod {owner} declares invalid identifier {id:?}"))?;
    let namespace = parsed.namespace();
    if !owns_namespace(owner, namespace) {
        return Err(format!(
            "mod {owner} declares {id:?} in unowned namespace {namespace:?}"
        ));
    }
    Ok(id)
}



impl crate::RegistrationContext {
    pub(crate) fn refuse_declaration(&self, reason: String) {
        self.declaration_refusals
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(reason);
    }
}

/// The engine consumes block/item declaration ownership refusals before loading
/// resources. Guest dispatch retains invalid output so the owning host boundary
/// sees and refuses it as well, rather than silently dropping a declaration.
// hidden: engine admission plumbing, re-exported through __private for the loader.
#[doc(hidden)]
pub fn take_declaration_refusals(context: &crate::RegistrationContext) -> Vec<String> {
    std::mem::take(
        &mut *context
            .declaration_refusals
            .lock()
            .unwrap_or_else(|p| p.into_inner()),
    )
}


