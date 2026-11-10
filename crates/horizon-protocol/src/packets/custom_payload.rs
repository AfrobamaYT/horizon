//! Directional common custom-payload codecs for configuration and play.
use crate::{
    Error, Result,
    buf::{Codec, Ident, Reader, Writer},
};

/// Exact application bytes following a namespaced channel identifier.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomPayload {
    pub channel: String,
    pub data: Vec<u8>,
}

/// Details of a directional payload size refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayloadTooLarge {
    pub id: String,
    pub size: usize,
    pub limit: usize,
}

/// ServerboundCustomPayloadPacket.MAX_PAYLOAD_SIZE in vanilla 26.3.
pub const SERVERBOUND_LIMIT: usize = 32_767;
/// ClientboundCustomPayloadPacket.MAX_PAYLOAD_SIZE in vanilla 26.3.
pub const CLIENTBOUND_LIMIT: usize = 1_048_576;

/// Common custom payload with a directional body limit, excluding the identifier.
pub struct PayloadCodec<const LIMIT: usize>;
pub type ServerboundPayload = PayloadCodec<SERVERBOUND_LIMIT>;
pub type ClientboundPayload = PayloadCodec<CLIENTBOUND_LIMIT>;

impl<const LIMIT: usize> PayloadCodec<LIMIT> {
    fn check(channel: &str, size: usize) -> Result<()> {
        if size > LIMIT {
            return Err(Error::payload_too_large(channel, size, LIMIT));
        }
        Ok(())
    }
}

impl<const LIMIT: usize> Codec<CustomPayload> for PayloadCodec<LIMIT> {
    fn read(r: &mut Reader) -> Result<CustomPayload> {
        let channel = Ident::read(r)?;
        let size = r.remaining();
        Self::check(&channel, size)?;
        Ok(CustomPayload {
            channel,
            data: r.rest().to_vec(),
        })
    }

    fn validate(v: &CustomPayload) -> Result<()> {
        Self::check(&v.channel, v.data.len())
    }

    fn write(v: &CustomPayload, w: &mut Writer) {
        Ident::write(&v.channel, w);
        w.bytes(&v.data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids,
        packets::{configuration, play},
    };

    #[test]
    fn directional_limits_in_both_states() {
        check(32_767, |b| {
            configuration::Serverbound::decode(ids::configuration::serverbound::CUSTOM_PAYLOAD, b)
                .map(|_| ())
        });
        check(1_048_576, |b| {
            configuration::Clientbound::decode(ids::configuration::clientbound::CUSTOM_PAYLOAD, b)
                .map(|_| ())
        });
        check(32_767, |b| {
            play::Serverbound::decode(ids::play::serverbound::CUSTOM_PAYLOAD, b).map(|_| ())
        });
        check(1_048_576, |b| {
            play::Clientbound::decode(ids::play::clientbound::CUSTOM_PAYLOAD, b).map(|_| ())
        });
    }

    #[test]
    fn refusal_leaves_body_unconsumed_and_legal_bytes_stay_exact() {
        let mut body = Writer::new();
        body.string("test:opaque");
        body.bytes(&vec![0x83; 32_768]);
        let mut reader = Reader::new(&body.data);
        assert!(ServerboundPayload::read(&mut reader).is_err());
        assert_eq!(reader.remaining(), 32_768);
        body.data.pop();
        let packet = ServerboundPayload::read(&mut Reader::new(&body.data)).unwrap();
        assert_eq!(packet.channel, "test:opaque");
        assert_eq!(packet.data, vec![0x83; 32_767]);
    }

    #[test]
    fn checked_encoding_applies_both_limits_in_both_states() {
        for (limit, encode) in [
            (
                32_767,
                (|p| configuration::Serverbound::CustomPayload(p).try_encode())
                    as fn(CustomPayload) -> Result<Vec<u8>>,
            ),
            (1_048_576, |p| {
                configuration::Clientbound::CustomPayload(p).try_encode()
            }),
            (32_767, |p| play::Serverbound::CustomPayload(p).try_encode()),
            (1_048_576, |p| {
                play::Clientbound::CustomPayload(p).try_encode()
            }),
        ] {
            let mut payload = CustomPayload {
                channel: "test:opaque".into(),
                data: vec![0x83; limit],
            };
            assert!(encode(payload.clone()).is_ok());
            payload.data.push(0);
            assert!(encode(payload).unwrap_err().0.contains("PayloadTooLarge"));
        }
    }

    fn check(limit: usize, decode: impl Fn(&[u8]) -> Result<()>) {
        let mut body = Writer::new();
        body.string("test:opaque");
        body.bytes(&vec![0x83; limit]);
        decode(&body.data).unwrap();
        body.data.push(0);
        let error = decode(&body.data).unwrap_err();
        assert!(error.0.contains("PayloadTooLarge"));
        assert!(error.0.contains("test:opaque"));
        assert!(error.0.contains(&format!("size: {}", limit + 1)));
        assert!(error.0.contains(&format!("limit: {limit}")));
    }
}
