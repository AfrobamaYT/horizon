//! Read-only client session and local-player status declarations.

pub trait Helpers {
    /// The logged-in client's connected server. None outside a client world;
    /// missing display names and memory addresses remain explicit None fields.
    #[notes = r#"## Client session and local-player status

`ClientHost::client_session()` supplies read-only `client_server()`,
`client_language()` and `client_frame_stats()`. All return `None` outside a
logged-in client world. These reads use existing local state and send nothing.
`ClientServer` keeps missing names and addresses explicit: its `name` is the
integrated world's display name, or `None` for an external connection (the game
retains the player name, not a server display name). Its `address` is absent for
memory connections; friend-invite secrets are redacted. `integrated_local`
means this window actually owns that integrated server, not that the address
looks like localhost. A splitscreen guest does not own the server.

`client_language()` reads the applied language, not a pending options choice.
Use the existing `ClientHost::is_paused()` for the applied pause state.
`ClientFrameStats` reports the game's most recently computed `fps`, lifetime
`rendered_frames` (menus included), and the newest completed frame's wall
interval as `frame_time_ns` (including waits). The interval is `None` until a
sample exists or after the chart is reset. Reading it starts no new recording.

`PlayerRef::block_breaking(&dyn ClientHost)` and `ambient_mood(&dyn ClientHost)`
only accept the current local player's handle. Stale, fake and nonlocal handles
return `None`, as does a disconnected client. `BlockBreakingState::Idle` is
instead a valid local player who is not breaking a block. `Breaking` gives the
target position, accumulated predicted progress and optional visible crack
stage (0 through 9; absent at zero progress). The stage uses the same
calculation as the world overlay. Mood is the existing cave ambience value,
normally 0 through 1, not a server-side measurement."#]
    #[helper(
        side = "client",
        service = "client_session",
        target = "horizon_client::game::native_client_session::Game<'_>::native_client_server"
    )]
    fn client_server();
    /// The applied language code, not a pending options selection. None outside
    /// a logged-in client world. Read is_paused for the applied pause state.
    #[helper(
        side = "client",
        service = "client_session",
        target = "horizon_client::game::native_client_session::Game<'_>::native_client_language"
    )]
    fn client_language();
    /// This window's existing local frame counters; None outside a logged-in
    /// client world. No history is collected by this read and nothing is sent.
    #[helper(
        side = "client",
        service = "client_session",
        target = "horizon_client::game::native_client_session::Game<'_>::native_client_frame_stats"
    )]
    fn client_frame_stats();
    /// The local player's predicted block-breaking state. None for stale,
    /// nonlocal or fake handles and outside a logged-in client world; Idle
    /// means a valid local player is not currently breaking a block.
    #[helper(
        side = "client",
        receiver = "player",
        target = "horizon_client::game::native_client_session::Game<'_>::native_player_block_breaking"
    )]
    fn player_block_breaking();
    /// The local player's current cave ambience mood (normally 0 through 1),
    /// not a server value. None for stale, nonlocal or fake handles and outside
    /// a logged-in client world.
    #[helper(
        side = "client",
        receiver = "player",
        target = "horizon_client::game::native_client_session::Game<'_>::native_player_ambient_mood"
    )]
    fn player_ambient_mood();
}
