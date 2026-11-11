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
///
/// Unions have no payload form:
/// ```compile_fail
/// #[derive(mod_api::PayloadCodec)]
/// union Raw { a: u8, b: u32 }
/// ```
///
/// Unknown type attributes are refused:
/// ```compile_fail
/// #[derive(mod_api::PayloadCodec)]
/// #[payload(with = other)]
/// struct Unknown;
/// ```
///
/// Field attributes cannot silently override encoding:
/// ```compile_fail
/// #[derive(mod_api::PayloadCodec)]
/// struct Bad { #[payload(with = other)] value: u8 }
/// ```
///
/// Variant attributes are also refused:
/// ```compile_fail
/// #[derive(mod_api::PayloadCodec)]
/// enum Bad { #[payload(with = other)] A }
/// ```
pub use horizon_mod_api_macros::PayloadCodec;

#[cfg(test)]
mod derive_shapes {
    use super::{decode_payload, encode_payload};
    use crate as contract;

    // Deliberately shadow prelude types and result constructors.
    #[allow(dead_code)]
    struct Result;
    #[allow(dead_code)]
    struct Option;
    #[allow(dead_code)]
    struct Ok;
    #[allow(dead_code)]
    struct Err;

    #[derive(Debug, PartialEq, horizon_mod_api_macros::PayloadCodec)]
    #[payload(crate = contract)]
    struct Tuple(u8);

    #[derive(Debug, PartialEq, horizon_mod_api_macros::PayloadCodec)]
    struct Unit;

    #[derive(Debug, PartialEq, horizon_mod_api_macros::PayloadCodec)]
    #[non_exhaustive]
    struct Generic<T, const N: usize>
    where
        T: Clone,
    {
        r#type: T,
    }

    #[derive(Debug, PartialEq, horizon_mod_api_macros::PayloadCodec)]
    enum Choice {
        One,
        Two(u8),
    }

    #[test]
    fn shapes_compile_and_match_independent_bytes() {
        assert_eq!(encode_payload(&Tuple(9), 1).unwrap(), [9]);
        assert_eq!(decode_payload::<Tuple>(&[9]).unwrap(), Tuple(9));
        assert_eq!(encode_payload(&Unit, 0).unwrap(), []);
        assert_eq!(decode_payload::<Unit>(&[]).unwrap(), Unit);
        let value = Generic::<u8, 3> { r#type: 7 };
        assert_eq!(encode_payload(&value, 1).unwrap(), [7]);
        assert_eq!(decode_payload::<Generic<u8, 3>>(&[7]).unwrap(), value);
        assert_eq!(decode_payload::<Choice>(&[1, 5]).unwrap(), Choice::Two(5));
        assert!(decode_payload::<Choice>(&[2]).is_err());
    }
}
