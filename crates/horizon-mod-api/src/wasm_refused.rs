//! A value that can't cross between the game and a wasm mod, shared by both sides.

/// Text validation errors remain typed; malformed transport values trap or refuse a send.
#[derive(Debug)]
pub(crate) enum Refused {
    Transport(std::borrow::Cow<'static, str>),
    Text(crate::TextFormError),
}

impl Refused {
    /// A list of the wrong length, or a case or handler that doesn't exist.
    pub(crate) const SHAPE: Self =
        Self::Transport(std::borrow::Cow::Borrowed("a value of the wrong shape"));

    /// A call into the game made outside a call into a mod: the runtime's own error.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) const NO_CALLER: Self = Self::Transport(std::borrow::Cow::Borrowed(
        "a mod's import ran outside a call into it",
    ));

    /// A transport conversion refused, for the reason it gives.
    pub(crate) fn because(why: impl std::fmt::Display) -> Self {
        Self::Transport(why.to_string().into())
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(why) => f.write_str(why),
            Self::Text(why) => why.fmt(f),
        }
    }
}

impl std::error::Error for Refused {}
