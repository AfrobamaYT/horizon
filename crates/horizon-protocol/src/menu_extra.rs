//! Menu extra data: what a Horizon server tells a Horizon client about a
//! native mod's menu type its block opened, just before the
//! `ClientboundOpenScreenPacket` of the same window (docs/native-mods.md,
//! "Menu extra data"). NeoForge's counterpart is `openMenu` with an extra-data
//! writer, read by the menu type's `IContainerFactory`.
//!
//! A payload is the window id, the block's position if there is one (a
//! boolean, then the three axes), then the bytes the mod wrote (empty when it
//! wrote none). Clientbound only.
use crate::{
    Error, Result,
    buf::{Reader, Writer},
};

/// The custom payload channel.
pub const CHANNEL: &str = "horizon:menu_extra";
/// Bytes a mod may write at most.
pub const MAX_DATA: usize = 32_768;

/// A menu's extra data as it travels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// The window id the `OpenScreen` that follows carries.
    pub window: i32,
    /// The block that opened the menu; None for a menu opened without one.
    pub pos: Option<[i32; 3]>,
    pub data: Vec<u8>,
}

/// A message's payload; an error for data past [`MAX_DATA`].
pub fn encode(message: &Message) -> Result<Vec<u8>> {
    if message.data.len() > MAX_DATA {
        return Err(Error::new("menu extra: data too big"));
    }
    let mut w = Writer::new();
    w.varint(message.window);
    w.bool(message.pos.is_some());
    message.pos.iter().flatten().for_each(|&axis| w.i32(axis));
    w.byte_array(&message.data);
    Ok(w.data)
}

/// A payload's message; an error for one that breaks a cap.
pub fn decode(data: &[u8]) -> Result<Message> {
    let mut r = Reader::new(data);
    let window = r.varint()?;
    let pos = if r.bool()? {
        Some([r.i32()?, r.i32()?, r.i32()?])
    } else {
        None
    };
    let data = r.byte_array(MAX_DATA)?;
    if r.remaining() > 0 {
        return Err(Error::new("menu extra: trailing bytes"));
    }
    Ok(Message { window, pos, data })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_round_trips() {
        for (pos, data) in [
            (Some([-30_000_000, -64, 29_999_999]), vec![]),
            (Some([0, 0, 0]), vec![0, 255, 7]),
            (None, vec![9; MAX_DATA]),
            (None, vec![1, 2, 3]),
        ] {
            let m = Message {
                window: 100,
                pos,
                data,
            };
            assert_eq!(decode(&encode(&m).unwrap()).unwrap(), m);
        }
    }

    #[test]
    fn caps_are_enforced_both_ways() {
        let big = Message {
            window: 1,
            pos: Some([0; 3]),
            data: vec![0; MAX_DATA + 1],
        };
        assert!(encode(&big).is_err());
        // Built by hand, as a server that skipped the check would.
        let mut w = Writer::new();
        w.varint(1);
        w.bool(false);
        w.byte_array(&vec![0; MAX_DATA + 1]);
        assert!(decode(&w.data).is_err());
        let mut ok = encode(&Message {
            window: 1,
            pos: Some([0; 3]),
            data: vec![1],
        })
        .unwrap();
        ok.push(0);
        assert!(decode(&ok).is_err(), "trailing bytes");
        assert!(decode(&[]).is_err());
        assert!(decode(&[1, 1, 0]).is_err(), "a cut position");
    }
}
