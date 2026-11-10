//! Self-refusal transport and the registration admission savepoint.
use super::*;

/// Engine admission plumbing; mods call `RegistrationContext::refuse` instead.
// hidden: the generated registrar call table transports self-refusals.
#[doc(hidden)]
pub trait RegistrationRefusalRegistrar {
    /// Deliver the current mod's refusal, without accepting an owner argument.
    fn registration_refusal(&mut self, reason: String);
}

/// Engine transport plumbing for the generated guest dispatch arm.
// hidden: drains the guest-local context after the registration callback.
#[doc(hidden)]
pub fn take_registration_refusal(cx: &crate::RegistrationContext) -> Option<String> {
    cx.self_refusal
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take()
}






