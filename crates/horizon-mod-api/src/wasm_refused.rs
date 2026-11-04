//! A value that can't cross between the game and a wasm mod, and why: one type for both sides
//! of the interface (`wasm`, the mod's; `wasm_host`, the game's).

/// A value that can't cross between the game and a mod (a tag past the limits, a list of the
/// wrong length), and why: the call traps with it, or isn't sent.
#[derive(Debug)]
pub(crate) struct Refused(std::borrow::Cow<'static, str>);

impl Refused {
    /// A value of the wrong shape: a list of the wrong length, a case or handler that doesn't
    /// exist.
    pub(crate) const SHAPE: Self = Self(std::borrow::Cow::Borrowed("a value of the wrong shape"));

    /// A call into the game made outside a call into a mod: the runtime's own error.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) const NO_CALLER: Self = Self(std::borrow::Cow::Borrowed(
        "a mod's import ran outside a call into it",
    ));

    /// A value its conversion refused, for the reason it gives.
    pub(crate) fn because(why: impl std::fmt::Display) -> Self {
        Self(why.to_string().into())
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refused {}
