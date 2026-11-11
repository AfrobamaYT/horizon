use super::*;

/// The acceptance matrix's payload: `counter=7`.
#[derive(Debug, PartialEq)]
struct Counter {
    counter: i32,
    labels: Vec<String>,
}

impl PayloadCodec for Counter {
    fn encode(&self, out: &mut PayloadWriter) {
        out.var_int(self.counter);
        self.labels.encode(out);
    }
    fn decode(input: &mut PayloadReader<'_>) -> Result<Self, PayloadDecodeError> {
        Ok(Self {
            counter: input.var_int()?,
            labels: Vec::decode(input)?,
        })
    }
}

/// A type that holds itself: `links` present options deep.
#[derive(Debug, PartialEq)]
struct Chain(Option<Box<Chain>>);

impl PayloadCodec for Chain {
    fn encode(&self, out: &mut PayloadWriter) {
        self.0.encode(out);
    }
    fn decode(input: &mut PayloadReader<'_>) -> Result<Self, PayloadDecodeError> {
        Option::decode(input).map(Self)
    }
}

fn chain(links: usize) -> Chain {
    (0..links).fold(Chain(None), |inner, _| Chain(Some(Box::new(inner))))
}

/// `links` true tags and the false that ends them.
fn chain_bytes(links: usize) -> Vec<u8> {
    let mut bytes = vec![1; links];
    bytes.push(0);
    bytes
}

/// A zero-sized entry that counts each decode, to show none ran.
#[derive(Debug, PartialEq)]
struct Counted;

thread_local! {
    static DECODED: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

impl PayloadCodec for Counted {
    fn encode(&self, _: &mut PayloadWriter) {}
    fn decode(_: &mut PayloadReader<'_>) -> Result<Self, PayloadDecodeError> {
        DECODED.set(DECODED.get() + 1);
        Ok(Self)
    }
}

/// counter 7, then one label "a".
const SEVEN: [u8; 4] = [7, 1, 1, b'a'];

fn seven() -> Counter {
    Counter {
        counter: 7,
        labels: vec!["a".to_owned()],
    }
}

#[test]
fn a_value_has_vanillas_network_bytes_and_reads_back() {
    assert_eq!(encode_payload(&seven(), 64), Ok(SEVEN.to_vec()));
    assert_eq!(decode_payload::<Counter>(&SEVEN), Ok(seven()));

    let mut out = PayloadWriter::new(64);
    out.var_int(300);
    out.var_int(-1);
    out.var_long(-1);
    out.i32(-2);
    out.i16(0x0102);
    out.bool(true);
    out.f32(1.0);
    out.uuid(0x0102);
    out.string("é");
    let bytes = out.finish().unwrap();
    let mut expected = vec![0xAC, 0x02];
    expected.extend([0xFF, 0xFF, 0xFF, 0xFF, 0x0F]);
    expected.extend([0xFF; 9]);
    expected.push(0x01);
    expected.extend([0xFF, 0xFF, 0xFF, 0xFE]);
    expected.extend([0x01, 0x02]);
    expected.push(1);
    expected.extend([0x3F, 0x80, 0, 0]);
    expected.extend([0; 14]);
    expected.extend([0x01, 0x02]);
    expected.extend([2, 0xC3, 0xA9]);
    assert_eq!(bytes, expected);

    let mut input = PayloadReader::new(&bytes);
    assert_eq!(input.var_int(), Ok(300));
    assert_eq!(input.var_int(), Ok(-1));
    assert_eq!(input.var_long(), Ok(-1));
    assert_eq!(input.i32(), Ok(-2));
    assert_eq!(input.i16(), Ok(0x0102));
    assert_eq!(input.bool(), Ok(true));
    assert_eq!(input.f32(), Ok(1.0));
    assert_eq!(input.uuid(), Ok(0x0102));
    assert_eq!(input.string().as_deref(), Ok("é"));
    assert_eq!(input.finish(), Ok(()));
}

/// The matrix row "Length -1, count 4097, depth 65, trailing byte": each is
/// refused, no entry is decoded, and the next body still reads as counter 7.
#[test]
fn hostile_bodies_are_refused_before_anything_is_reserved() {
    // A count of -1, then a count of 4097, each followed by nothing.
    let negative = [0xFF, 0xFF, 0xFF, 0xFF, 0x0F];
    let over = [0x81, 0x20];
    DECODED.set(0);
    assert_eq!(
        decode_payload::<Vec<Counted>>(&negative),
        Err(PayloadDecodeError::NegativeLength { length: -1 })
    );
    assert_eq!(
        decode_payload::<Vec<Counted>>(&over),
        Err(PayloadDecodeError::LengthOverLimit {
            length: 4097,
            limit: 4096
        })
    );
    assert_eq!(DECODED.get(), 0);
    // The same two lengths on a string.
    assert_eq!(
        decode_payload::<String>(&negative),
        Err(PayloadDecodeError::NegativeLength { length: -1 })
    );
    assert_eq!(
        decode_payload::<String>(&over),
        Err(PayloadDecodeError::LengthOverLimit {
            length: 4097,
            limit: 4096
        })
    );

    assert_eq!(decode_payload(&chain_bytes(64)), Ok(chain(64)));
    assert_eq!(
        decode_payload::<Chain>(&chain_bytes(65)),
        Err(PayloadDecodeError::TooDeep)
    );

    let mut trailing = SEVEN.to_vec();
    trailing.push(0);
    assert_eq!(
        decode_payload::<Counter>(&trailing),
        Err(PayloadDecodeError::Trailing { bytes: 1 })
    );

    assert_eq!(decode_payload::<Counter>(&SEVEN), Ok(seven()));
}

#[test]
fn a_list_is_charged_its_decoded_size_before_it_is_reserved() {
    // 4096 absent options: 2 + 4096 bytes of body, 4096 * 16 bytes decoded.
    let mut body = vec![0x80, 0x20];
    body.extend([0; 4096]);
    let entry = std::mem::size_of::<Option<i64>>();
    let read = |budget| {
        let mut input = PayloadReader::with_budget(&body, budget);
        Vec::<Option<i64>>::decode(&mut input).map(|list| list.len())
    };
    assert_eq!(read(4096 * entry), Ok(4096));
    assert_eq!(
        read(4096 * entry - 1),
        Err(PayloadDecodeError::BudgetExhausted)
    );
    // Strings draw on the same budget: two of 3 bytes do not fit in 5.
    let strings = [2, 3, b'a', b'b', b'c', 3, b'd', b'e', b'f'];
    let mut input = PayloadReader::with_budget(&strings, 2 * std::mem::size_of::<String>() + 5);
    assert_eq!(
        Vec::<String>::decode(&mut input),
        Err(PayloadDecodeError::BudgetExhausted)
    );
}

#[test]
fn malformed_values_are_named() {
    use PayloadDecodeError::*;
    assert_eq!(decode_payload::<i32>(&[0, 0, 0]), Err(UnexpectedEnd));
    // A string of 3 bytes with 2 behind its length.
    assert_eq!(decode_payload::<String>(&[3, b'a', b'b']), Err(UnexpectedEnd));
    assert_eq!(decode_payload::<String>(&[2, 0xC3, 0x28]), Err(Utf8));
    assert_eq!(
        decode_payload::<bool>(&[2]),
        Err(InvalidDiscriminant { value: 2 })
    );
    assert_eq!(
        decode_payload::<Option<u8>>(&[9, 1]),
        Err(InvalidDiscriminant { value: 9 })
    );
    assert_eq!(decode_payload::<VarInt>(&[0x80; 6]), Err(VarIntTooLong));
    assert_eq!(decode_payload::<VarLong>(&[0x80; 11]), Err(VarIntTooLong));
    assert_eq!(decode_payload::<VarInt>(&[0x80, 0x80]), Err(UnexpectedEnd));
    let nan = f64::NAN.to_be_bytes();
    assert_eq!(PayloadReader::new(&nan).finite_f64(), Err(NonFinite));
    assert!(PayloadReader::new(&nan).f64().is_ok_and(f64::is_nan));
    // A byte array bounded by its own schema, not by the list bound.
    let mut input = PayloadReader::new(&[3, 1, 2, 3]);
    assert_eq!(
        input.bytes(2),
        Err(LengthOverLimit {
            length: 3,
            limit: 2
        })
    );
    assert_eq!(PayloadReader::new(&[3, 1, 2, 3]).bytes(3), Ok(&[1, 2, 3][..]));
}

#[test]
fn an_encode_past_a_bound_writes_nothing() {
    use PayloadEncodeError::*;
    // 4 bytes fit in 4, not in 3.
    assert_eq!(encode_payload(&seven(), 4), Ok(SEVEN.to_vec()));
    assert_eq!(encode_payload(&seven(), 3), Err(TooLarge { limit: 3 }));
    assert_eq!(
        encode_payload(&"a".repeat(4097), usize::MAX),
        Err(LengthOverLimit {
            length: 4097,
            limit: 4096
        })
    );
    assert_eq!(
        encode_payload(&vec![0u8; 4097], usize::MAX),
        Err(LengthOverLimit {
            length: 4097,
            limit: 4096
        })
    );
    assert_eq!(
        encode_payload(&chain(64), usize::MAX),
        Ok(chain_bytes(64))
    );
    assert_eq!(encode_payload(&chain(65), usize::MAX), Err(TooDeep));
    let mut out = PayloadWriter::new(64);
    out.finite_f32(f32::INFINITY);
    // The first failure stays, and later writes add nothing.
    out.u8(1);
    assert_eq!(out.finish(), Err(NonFinite));
}
