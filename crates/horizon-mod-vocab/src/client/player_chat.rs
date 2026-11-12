//! A player chat message as its sender signed it (`PlayerChatMessage`), for
//! `client_chat_received`.
#[allow(unused_imports)]
use crate::*;

/// NeoForge's `ClientChatReceivedEvent.Player.getPlayerChatMessage()`: the
/// message as the player sent it, beside the decorated line the chat shows.
/// The signature bytes are left out; `signed` says whether one validated.
#[derive(Clone, Debug, PartialEq)]
// exhaustive: the game builds it with a literal; mods only read it
#[derive(horizon_mod_api_macros::Wire)]
pub struct PlayerChatInfo {
    /// `signedBody().content()`: the plain text the player typed.
    pub body: String,
    /// The message carries a signature that validated (`hasSignature`).
    pub signed: bool,
    /// `timeStamp()`, in milliseconds since the epoch.
    pub timestamp_ms: i64,
    /// `salt()`.
    pub salt: i64,
    /// `unsignedContent()`: the server's replacement for `body`, if it set one.
    pub unsigned_content: Option<Text>,
    /// `filterMask()`.
    pub filter: ChatFilter,
}

/// `FilterMask`: which characters of `body` the server's filter hides.
#[derive(Clone, Debug, PartialEq, Eq)]
// exhaustive: vanilla's three filter mask types
#[derive(horizon_mod_api_macros::Wire)]
pub enum ChatFilter {
    /// `PASS_THROUGH`: nothing is hidden.
    PassThrough,
    /// `FULLY_FILTERED`: the whole message is hidden.
    FullyFiltered,
    /// `PARTIALLY_FILTERED`: the hidden characters, as a `BitSet`'s words
    /// (bit `i` of word `i / 64` hides character `i`).
    Partial(Vec<u64>),
}
