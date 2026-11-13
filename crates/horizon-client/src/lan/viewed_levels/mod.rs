//! Viewed levels, server side (#3716): levels besides their own that players' clients keep, so
//! that the server can send them those levels' chunks and updates on `horizon:levels`
//! (`horizon_protocol::viewed_levels`).
pub mod capability;
