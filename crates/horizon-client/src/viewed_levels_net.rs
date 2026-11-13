//! Viewed levels, client side (#3716): the answer to a Horizon server's `horizon:levels` query in
//! configuration (`horizon_protocol::viewed_levels`), sent before the client acknowledges
//! `FinishConfiguration`. The server sends a client nothing more on the channel than its answer
//! allows.
use horizon_protocol::{packets::configuration as cfg, viewed_levels};

/// Levels besides its own this client can view. It keeps one world, so none.
const MAX_LEVELS: u8 = 0;

/// Whether `payload` is the server's viewed-levels query.
pub fn asked(payload: &cfg::CustomPayload) -> bool {
    payload.channel == viewed_levels::CHANNEL
}

/// The answer to the query `data`, or the reason that closes the connection.
pub fn answer(data: &[u8]) -> Result<cfg::Serverbound, String> {
    viewed_levels::read_query(data).map_err(|error| error.to_string())?;
    Ok(cfg::Serverbound::CustomPayload(cfg::CustomPayload {
        channel: viewed_levels::CHANNEL.into(),
        data: viewed_levels::answer(MAX_LEVELS),
    }))
}
