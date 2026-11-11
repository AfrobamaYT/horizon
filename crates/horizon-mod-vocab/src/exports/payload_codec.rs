pub use content::payload_codec::{
    PAYLOAD_DECODE_BUDGET, PAYLOAD_MAX_DEPTH, PAYLOAD_MAX_LIST_ENTRIES, PAYLOAD_MAX_STRING_BYTES,
    PayloadCodec, PayloadDecodeError, PayloadEncodeError, PayloadReader, PayloadWriter, VarInt,
    VarLong, decode_payload, encode_payload,
};

/// Derive [`PayloadCodec`] from a type's fields.
///
/// A struct is its fields in declaration order with nothing between them. An
/// enum is a VarInt tag, the variant's position counted from 0 whatever its
/// discriminant, then that variant's fields; a tag no variant has is
/// [`PayloadDecodeError::InvalidDiscriminant`]. Every field type implements
/// [`PayloadCodec`], so each keeps its own bounds; a field that needs another
/// bound or a finite float gets a type with a hand-written codec. A derived
/// type adds no nesting level of its own: the lists, options and boxes inside
/// it are what count toward [`PAYLOAD_MAX_DEPTH`].
///
/// ```
/// use mod_api::{PayloadCodec, VarInt, decode_payload, encode_payload};
///
/// #[derive(Debug, PartialEq, PayloadCodec)]
/// struct Counter {
///     counter: VarInt,
/// }
///
/// let body = encode_payload(&Counter { counter: VarInt(7) }, 16).unwrap();
/// assert_eq!(body, [7]);
/// assert_eq!(decode_payload(&body), Ok(Counter { counter: VarInt(7) }));
/// ```
///
/// A field without a payload form does not compile:
/// ```compile_fail
/// struct NoCodec;
/// #[derive(mod_api::PayloadCodec)]
/// struct Missing { value: NoCodec }
/// ```
///
/// Neither does an enum with no variants:
/// ```compile_fail
/// #[derive(mod_api::PayloadCodec)]
/// enum Never {}
/// ```
pub use horizon_mod_api_macros::PayloadCodec;
