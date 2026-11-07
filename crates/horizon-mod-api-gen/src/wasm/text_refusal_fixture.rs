// Runtime fixture for production-generated adapters; the wire shape is deliberately small.
extern crate self as wasmtime;
type Result<T, E = Error> = std::result::Result<T, E>;
#[derive(Debug)]
struct Error(Box<dyn std::any::Any>);
impl Error {
    fn downcast<T: 'static>(self) -> Result<T, Self> {
        self.0.downcast::<T>().map(|x| *x).map_err(Self)
    }
}
impl From<Refused> for Error {
    fn from(v: Refused) -> Self {
        Self(Box::new(v))
    }
}
#[derive(Debug, PartialEq)]
pub enum TextFormError {
    TooDeep,
    Malformed,
    Invalid,
}
impl std::fmt::Display for TextFormError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TextFormError {}
// REFUSED
#[derive(Debug)]
pub struct Text(u32);
impl TryFrom<&Text> for u32 {
    type Error = TextFormError;
    fn try_from(t: &Text) -> Result<Self, Self::Error> {
        match t.0 {
            513 => Err(TextFormError::TooDeep),
            600 => Err(TextFormError::Malformed),
            700 => Err(TextFormError::Invalid),
            n => Ok(n),
        }
    }
}
impl TryFrom<u32> for Text {
    type Error = TextFormError;
    fn try_from(n: u32) -> Result<Self, Self::Error> {
        u32::try_from(&Text(n)).map(Text)
    }
}
pub struct TextBatch {
    pub values: Vec<Option<Text>>,
    pub shape: [u32; 13],
}
trait TextProbe {
    fn direct(&mut self, input: Text) -> Result<u32, TextFormError>;
    fn batch(&mut self, input: TextBatch) -> Result<u32, TextFormError>;
    fn output(&mut self) -> Result<Text, TextFormError>;
}
#[derive(Default)]
struct Probe {
    calls: u32,
}
impl TextProbe for Probe {
    fn direct(&mut self, input: Text) -> Result<u32, TextFormError> {
        self.calls += 1;
        Ok(input.0)
    }
    fn batch(&mut self, input: TextBatch) -> Result<u32, TextFormError> {
        self.calls += 1;
        Ok(input.values.len() as u32)
    }
    fn output(&mut self) -> Result<Text, TextFormError> {
        self.calls += 1;
        Ok(Text(513))
    }
}
fn expect<T>(value: Result<T, Refused>) -> T {
    value.unwrap()
}
fn array<T, const N: usize>(items: Vec<T>) -> Result<[T; N], Refused> {
    items.try_into().map_err(|_| Refused::SHAPE)
}
#[derive(Default)]
struct Keep;
mod horizon {
    pub mod mod_api {
        pub mod types {
            pub type Text = u32;
            #[derive(Debug, PartialEq)]
            pub enum TextFormError {
                TooDeep,
                Malformed,
                Invalid,
            }
            pub struct TextBatch {
                pub values: Vec<Option<Text>>,
                pub shape: Vec<u32>,
            }
        }
    }
}
use horizon::mod_api::types as wire;
mod imports {
    use super::*;
    pub fn direct(input: u32) -> Result<u32, wire::TextFormError> {
        assert!(input <= 512);
        Ok(input)
    }
    pub fn batch(input: &wire::TextBatch) -> Result<u32, wire::TextFormError> {
        assert!(input.values.iter().flatten().all(|n| *n <= 512));
        Ok(input.values.len() as u32)
    }
    pub fn output() -> Result<u32, wire::TextFormError> {
        Ok(513)
    }
}
// GENERATED
#[test]
fn guest_refuses_before_import_and_preserves_variants() {
    let mut g = guest::Guest;
    assert_eq!(g.direct(Text(512)), Ok(512));
    for (n, why) in [
        (513, TextFormError::TooDeep),
        (600, TextFormError::Malformed),
        (700, TextFormError::Invalid),
    ] {
        assert_eq!(g.direct(Text(n)), Err(why));
    }
    assert_eq!(
        g.batch(TextBatch {
            values: vec![None, Some(Text(513))],
            shape: [0; 13],
        }),
        Err(TextFormError::TooDeep)
    );
    assert!(matches!(g.output(), Err(TextFormError::TooDeep)));
}
#[test]
fn host_refuses_before_write_and_preserves_variants() {
    let mut p = Probe::default();
    assert_eq!(host::ProbeCalls::direct(&mut p, 512).unwrap(), Ok(512));
    for (n, why) in [
        (513, wire::TextFormError::TooDeep),
        (600, wire::TextFormError::Malformed),
        (700, wire::TextFormError::Invalid),
    ] {
        assert_eq!(host::ProbeCalls::direct(&mut p, n).unwrap(), Err(why));
    }
    assert_eq!(
        host::ProbeCalls::batch(
            &mut p,
            wire::TextBatch {
                values: vec![Some(513)],
                shape: vec![0; 13],
            }
        )
        .unwrap(),
        Err(wire::TextFormError::TooDeep)
    );
    assert_eq!(p.calls, 1);
    assert!(matches!(
        host::ProbeCalls::output(&mut p).unwrap(),
        Err(wire::TextFormError::TooDeep)
    ));
}
#[test]
fn unrelated_transport_errors_stay_transport_errors() {
    let mut p = Probe::default();
    let why = host::ProbeCalls::batch(
        &mut p,
        wire::TextBatch {
            values: vec![Some(512)],
            shape: vec![0; 12],
        },
    )
    .unwrap_err();
    assert!(matches!(
        why.downcast::<Refused>().unwrap(),
        Refused::Transport(_)
    ));
    assert_eq!(p.calls, 0);
    assert!(matches!(Refused::because("wire"), Refused::Transport(_)));
}
