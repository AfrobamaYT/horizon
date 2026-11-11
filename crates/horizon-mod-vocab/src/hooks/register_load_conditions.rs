//! Load-phase data-file condition registration.
pub trait Hooks {
    /// Register named conditions once at mod load, before server data admission.
    /// Names must belong to the registering mod's namespace. Native and wasm
    /// handlers use the same registration and evaluation route. Registering one
    /// name owns that namespace; an unknown condition in it refuses data reload.
    /// Unknown conditions in other namespaces omit their file with one warning.
    #[hook(
        side = "server",
        contract = "collect",
        out = "conditions",
        context = "registration"
    )]
    fn register_load_conditions();
}
