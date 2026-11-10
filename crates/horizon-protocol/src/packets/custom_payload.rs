//! Directional discard-fallback limits for configuration and play custom payloads.
//! Existing registered engine channels retain their own codec constraints.
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

/// Common payload with a directional unknown-id limit, excluding the identifier.
/// Registered payloads use their own codec constraints, not this discard cap.
pub struct PayloadCodec<const LIMIT: usize>;
pub type ServerboundPayload = PayloadCodec<SERVERBOUND_LIMIT>;
pub type ClientboundPayload = PayloadCodec<CLIENTBOUND_LIMIT>;

/// The single registered-codec lookup seam. The transport registration work
/// replaces this engine-only classification with its frozen codec lookup.
/// These channels already have engine decoders; they are not discard payloads.
fn uses_registered_payload_codec(channel: &str) -> bool {
    matches!(
        channel,
        "minecraft:brand"
            | crate::quiet::CHANNEL
            | crate::mod_net::CHANNEL
            | crate::mod_list::CHANNEL
            | crate::attachment_sync::CHANNEL
            | crate::menu_extra::CHANNEL
            | crate::anvil_cost::CHANNEL
            | "horizon:lod"
            | "horizon:block_registry"
            | "horizon:recipe_content"
            | "horizon:recipe_transfer"
            | "horizon:recipe_book_settings"
    )
}

impl<const LIMIT: usize> PayloadCodec<LIMIT> {
    fn check(channel: &str, size: usize) -> Result<()> {
        if !uses_registered_payload_codec(channel) && size > LIMIT {
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
        let body = r.rest();
        validate_brand(&channel, body)?;
        Ok(CustomPayload {
            channel,
            data: body.to_vec(),
        })
    }

    fn validate(v: &CustomPayload) -> Result<()> {
        Self::check(&v.channel, v.data.len())?;
        validate_brand(&v.channel, &v.data)
    }

    fn write(v: &CustomPayload, w: &mut Writer) {
        Ident::write(&v.channel, w);
        w.bytes(&v.data);
    }
}

fn validate_brand(channel: &str, body: &[u8]) -> Result<()> {
    if channel == "minecraft:brand" {
        let mut reader = Reader::new(body);
        reader.string(32_767)?;
        if reader.remaining() != 0 {
            return Err(Error::new("brand payload has trailing bytes"));
        }
    }
    Ok(())
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
    fn registered_codec_seam_exempts_only_existing_engine_ids() {
        for channel in [
            "minecraft:brand",
            "horizon:quiet_commands",
            "horizon:mod_net",
            "horizon:mod_list",
            "horizon:attachment_sync",
            "horizon:menu_extra",
            "horizon:anvil_cost",
            "horizon:lod",
            "horizon:block_registry",
            "horizon:recipe_content",
            "horizon:recipe_transfer",
            "horizon:recipe_book_settings",
        ] {
            assert!(uses_registered_payload_codec(channel), "{channel}");
            assert!(ServerboundPayload::check(channel, 32_768).is_ok());
            assert!(ClientboundPayload::check(channel, 1_048_577).is_ok());
        }
        for channel in [
            "horizon:not_registered",
            "farmersdelight:flip_skillet",
            "minecraft:unknown",
            "other:mod_net",
        ] {
            assert!(!uses_registered_payload_codec(channel), "{channel}");
            assert!(ServerboundPayload::check(channel, 32_768).is_err());
            assert!(ClientboundPayload::check(channel, 1_048_577).is_err());
        }
    }

    #[test]
    fn registered_brand_is_not_bounded_by_discard_capacity() {
        // A legal UTF-8 brand can exceed 32,767 bytes while staying below
        // FriendlyByteBuf's 32,767 UTF-16 code-unit string limit.
        let mut brand = Writer::new();
        brand.string(&"é".repeat(16_384));
        let payload = CustomPayload {
            channel: "minecraft:brand".into(),
            data: brand.data,
        };
        assert!(payload.data.len() > 32_767);
        for packet in [
            configuration::Serverbound::CustomPayload(payload.clone())
                .try_encode()
                .unwrap(),
            play::Serverbound::CustomPayload(payload)
                .try_encode()
                .unwrap(),
        ] {
            let mut reader = Reader::new(&packet);
            reader.varint().unwrap();
            assert!(ServerboundPayload::read(&mut reader).is_ok());
        }
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
