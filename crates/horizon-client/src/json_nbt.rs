//! JSON-to-NBT at the game/API boundary, matching 26.3 JsonOps -> NbtOps.
//!
//! Call once at load/reload for metadata, not on the tick. Numeric choice uses
//! the JSON number's decimal representation (serde_json has already parsed it).
//! JsonOps.convertTo chooses the smallest exact signed integer, otherwise an
//! exact float or a double. DynamicOps.convertList calls NbtOps.createList:
//! JSON arrays remain lists, including empty, numeric and heterogeneous arrays;
//! they do not use the typed-array collectors used by mergeToList.
use horizon_core::nbt::Tag;
use serde_json::{Number, Value};

/// JSON metadata refused by vanilla's JsonOps-to-NbtOps conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonNbtError {
    /// JsonOps.getStream/getMapValues expose nested JsonNull as Java null,
    /// which JsonOps.convertTo refuses (NullPointerException).
    NestedNull,
}
impl std::fmt::Display for JsonNbtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("JSON-to-NBT does not accept null inside a list or object")
    }
}
impl std::error::Error for JsonNbtError {}

/// Convert JSON metadata to the shared NBT vocabulary. Top-level null becomes
/// End; nested null is refused. The caller owns schema validation; this
/// conversion adds no schema or limits. Float-backed JSON numbers have already
/// been rounded by serde_json; their decimal spelling is the conversion input.
pub fn json_to_tag(value: &Value) -> Result<Tag, JsonNbtError> {
    convert(value, false)
}

fn convert(value: &Value, nested: bool) -> Result<Tag, JsonNbtError> {
    Ok(match value {
        Value::Null if nested => return Err(JsonNbtError::NestedNull),
        Value::Null => Tag::End,
        Value::Bool(value) => Tag::Byte(i8::from(*value)),
        Value::String(value) => Tag::String(value.as_str().into()),
        Value::Number(value) => number(value),
        Value::Array(values) => Tag::List(
            values
                .iter()
                .map(|v| convert(v, true))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(values) => Tag::Compound(
            values
                .iter()
                .map(|(key, value)| Ok((key.as_str().into(), convert(value, true)?)))
                .collect::<Result<_, JsonNbtError>>()?,
        ),
    })
}

fn number(number: &Number) -> Tag {
    // BigDecimal.longValueExact in JsonOps: do not round a fraction or a u64
    // through f64 into an apparently integral signed long.
    if let Some(value) = exact_long(&number.to_string()) {
        if let Ok(value) = i8::try_from(value) {
            return Tag::Byte(value);
        }
        if let Ok(value) = i16::try_from(value) {
            return Tag::Short(value);
        }
        if let Ok(value) = i32::try_from(value) {
            return Tag::Int(value);
        }
        return Tag::Long(value);
    }
    let value = number
        .as_f64()
        .expect("a JSON number is representable as f64");
    let float = value as f32;
    if f64::from(float) == value {
        Tag::Float(if float == 0.0 { 0 } else { float.to_bits() })
    } else {
        Tag::Double(if value == 0.0 { 0 } else { value.to_bits() })
    }
}

/// Decimal integer extraction without rounding. The input is Number's valid
/// decimal spelling; this is not a second JSON parser.
fn exact_long(decimal: &str) -> Option<i64> {
    let (mantissa, exponent) = decimal
        .split_once(['e', 'E'])
        .map_or((decimal, 0), |(m, e)| {
            (m, e.parse::<i32>().expect("JSON exponent"))
        });
    let negative = mantissa.starts_with('-');
    let mantissa = mantissa.trim_start_matches('-');
    let fractional = mantissa.split_once('.').map_or(0, |(_, f)| f.len() as i32);
    let mut digits: Vec<u8> = mantissa.bytes().filter(|b| *b != b'.').collect();
    let mut scale = exponent - fractional;
    while scale < 0 && digits.last() == Some(&b'0') {
        digits.pop();
        scale += 1;
    }
    if digits.iter().all(|b| *b == b'0') {
        return Some(0);
    }
    if !(0..=18).contains(&scale) {
        return None;
    }
    // Accumulate negative so i64::MIN is representable as well.
    let mut value = 0i64;
    for digit in digits {
        value = value
            .checked_mul(10)?
            .checked_sub(i64::from(digit - b'0'))?;
    }
    for _ in 0..scale {
        value = value.checked_mul(10)?;
    }
    if negative {
        Some(value)
    } else {
        value.checked_neg()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn tag(value: &Value) -> Tag {
        json_to_tag(value).unwrap()
    }

    #[test]
    fn refuses_nested_null_without_inventing_end_payloads() {
        assert_eq!(json_to_tag(&Value::Null), Ok(Tag::End));
        for value in [json!([null]), json!({"x": null}), json!({"x": [1, null]})] {
            assert_eq!(json_to_tag(&value), Err(JsonNbtError::NestedNull));
        }
    }

    #[test]
    fn numeric_boundaries_do_not_round_through_double() {
        for (source, expected) in [
            ("-128", Tag::Byte(-128)),
            ("127", Tag::Byte(127)),
            ("128", Tag::Short(128)),
            ("-129", Tag::Short(-129)),
            ("32767", Tag::Short(32767)),
            ("32768", Tag::Int(32768)),
            ("-32769", Tag::Int(-32769)),
            ("2147483647", Tag::Int(i32::MAX)),
            ("2147483648", Tag::Long(2147483648)),
            ("-9223372036854775808", Tag::Long(i64::MIN)),
            ("9223372036854775807", Tag::Long(i64::MAX)),
            (
                "9223372036854775808",
                Tag::Float((9223372036854775808.0f32).to_bits()),
            ),
            ("1.0", Tag::Byte(1)),
            ("1e3", Tag::Short(1000)),
            ("1.5", Tag::Float(1.5f32.to_bits())),
            ("0.1", Tag::Double(0.1f64.to_bits())),
            ("-0.0", Tag::Byte(0)),
        ] {
            assert_eq!(
                tag(&serde_json::from_str::<Value>(source).unwrap()),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn json_arrays_are_never_nbt_arrays_or_compound_wrappers() {
        assert_eq!(tag(&json!([])), Tag::List(vec![]));
        assert_eq!(
            tag(&json!([1, 2])),
            Tag::List(vec![Tag::Byte(1), Tag::Byte(2)])
        );
        assert_eq!(
            tag(&json!([32768, 2147483648i64])),
            Tag::List(vec![Tag::Int(32768), Tag::Long(2147483648)])
        );
        assert_eq!(
            tag(&json!([true, "x", {}])),
            Tag::List(vec![
                Tag::Byte(1),
                Tag::String("x".into()),
                Tag::compound([] as [(&str, Tag); 0])
            ])
        );
    }

    #[test]
    fn metadata_preserves_boolean_strings_and_nested_list() {
        let tag =
            tag(&json!({"custom": {"enabled": false, "note": "present"}, "lore": ["a", "b"]}));
        assert_eq!(
            tag.get("custom").unwrap().get("enabled"),
            Some(&Tag::Byte(0))
        );
        assert_eq!(
            tag.get("custom").unwrap().get("note"),
            Some(&Tag::String("present".into()))
        );
        assert_eq!(
            tag.get("lore"),
            Some(&Tag::List(vec![
                Tag::String("a".into()),
                Tag::String("b".into())
            ]))
        );
    }
}
