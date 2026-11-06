#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryRecord {
    pub amount: u32,
    pub block: BlockKey,
    pub effects: Vec<Option<EffectKey>>,
    pub choice: BoundaryChoice,
}
#[derive(Clone, Debug, PartialEq)]
pub enum BoundaryChoice {
    Absent,
    One(BlockKey),
    Record { effect: EffectKey },
}
#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryEvent {
    pub value: BoundaryRecord,
}
// END DECLARATIONS

extern crate self as wasmtime;
type Result<T, E = Refused> = std::result::Result<T, E>;
#[derive(Debug)]
pub struct Refused(String);
impl Refused {
    fn because(why: impl std::fmt::Display) -> Self {
        Self(why.to_string())
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockKey(u32);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectKey(u32);
// Independent fixture tables: valid slots deliberately differ from the transported numeric
// data. The production canonical request/mapping owners are exercised in the API tests.
impl BlockKey {
    fn from_slot(slot: u32) -> std::result::Result<Self, &'static str> {
        if slot < 2 {
            Ok(Self(slot))
        } else {
            Err("unknown BlockKey slot")
        }
    }
}
impl EffectKey {
    fn from_slot(slot: u32) -> std::result::Result<Self, &'static str> {
        if slot < 3 {
            Ok(Self(slot))
        } else {
            Err("unknown EffectKey slot")
        }
    }
}
mod horizon {
    pub mod mod_api {
        pub mod types {
            pub type BlockKey = u32;
            pub type EffectKey = u32;
            #[derive(Clone)]
            pub struct BoundaryRecord {
                pub amount: u32,
                pub block: u32,
                pub effects: Vec<Option<u32>>,
                pub choice: BoundaryChoice,
            }
            #[derive(Clone)]
            pub enum BoundaryChoice {
                Absent,
                One(u32),
                Record(BoundaryChoiceRecord),
            }
            #[derive(Clone)]
            pub struct BoundaryChoiceRecord {
                pub effect: u32,
            }
            pub struct BoundaryEvent {
                pub value: BoundaryRecord,
            }
            pub struct BoundaryEventOut {
                pub value: Option<BoundaryRecord>,
            }
        }
    }
}
use horizon::mod_api::types as wire;
#[derive(Default)]
struct Probe {
    calls: usize,
}
trait BoundaryProbe {
    fn block(&mut self, input: BlockKey) -> u32;
    fn effect(&mut self, input: EffectKey) -> u32;
    fn record(&mut self, input: BoundaryRecord) -> u32;
}
impl BoundaryProbe for Probe {
    fn block(&mut self, input: BlockKey) -> u32 {
        self.calls += 1;
        input.0 + 100
    }
    fn effect(&mut self, input: EffectKey) -> u32 {
        self.calls += 1;
        input.0 + 200
    }
    fn record(&mut self, input: BoundaryRecord) -> u32 {
        self.calls += 1;
        assert_eq!(input.block, BlockKey(1));
        assert_eq!(input.effects, [None, Some(EffectKey(2))]);
        input.amount
    }
}
struct Keep;
impl Default for Keep {
    fn default() -> Self {
        Self
    }
}
fn record() -> wire::BoundaryRecord {
    wire::BoundaryRecord {
        amount: u32::MAX,
        block: 1,
        effects: vec![None, Some(2)],
        choice: wire::BoundaryChoice::Absent,
    }
}
fn invalid_records() -> Vec<wire::BoundaryRecord> {
    let mut field = record();
    field.block = 2;
    let mut optional = record();
    optional.effects = vec![None, Some(3)];
    let mut tuple_case = record();
    tuple_case.choice = wire::BoundaryChoice::One(2);
    let mut record_case = record();
    record_case.choice = wire::BoundaryChoice::Record(wire::BoundaryChoiceRecord { effect: 3 });
    vec![field, optional, tuple_case, record_case]
}
// TESTS
#[test]
fn top_level_slots_and_numeric_data() {
    let mut probe = Probe::default();
    assert_eq!(Probe::block(&mut probe, 1).unwrap(), 101);
    assert_eq!(Probe::effect(&mut probe, 2).unwrap(), 202);
    for slot in [2, u32::MAX] {
        assert!(
            Probe::block(&mut probe, slot)
                .unwrap_err()
                .0
                .contains("unknown BlockKey")
        );
    }
    for slot in [3, u32::MAX] {
        assert!(
            Probe::effect(&mut probe, slot)
                .unwrap_err()
                .0
                .contains("unknown EffectKey")
        );
    }
    assert_eq!(probe.calls, 2, "refused imports never call the backend");
    assert_eq!(
        host::lower_block(&BlockKey(1)).unwrap(),
        1,
        "numeric transport, not a name"
    );
    assert_eq!(
        guest::block(u32::MAX).unwrap(),
        BlockKey(u32::MAX),
        "guest needs no registry table"
    );
}
#[test]
fn nested_imports_validate_before_execution() {
    let mut probe = Probe::default();
    for value in invalid_records() {
        assert!(Probe::record(&mut probe, value).is_err());
    }
    assert_eq!(probe.calls, 0);
    for choice in [
        wire::BoundaryChoice::Absent,
        wire::BoundaryChoice::One(0),
        wire::BoundaryChoice::Record(wire::BoundaryChoiceRecord { effect: 1 }),
    ] {
        let mut value = record();
        value.choice = choice;
        assert_eq!(Probe::record(&mut probe, value).unwrap(), u32::MAX);
    }
    assert_eq!(probe.calls, 3);
}
fn event() -> BoundaryEvent {
    BoundaryEvent {
        value: BoundaryRecord {
            amount: 8,
            block: BlockKey(0),
            effects: vec![],
            choice: BoundaryChoice::Absent,
        },
    }
}
#[test]
fn nested_hook_outputs_are_validated_before_assignment() {
    let mut event = event();
    let original = event.clone();
    for value in invalid_records() {
        assert!(host::write(&mut event, wire::BoundaryEventOut { value: Some(value) }).is_err());
        assert_eq!(event, original);
    }
    host::write(
        &mut event,
        wire::BoundaryEventOut {
            value: Some(record()),
        },
    )
    .unwrap();
    assert_eq!(event.value.amount, u32::MAX);
    assert_eq!(event.value.block, BlockKey(1));
    assert_eq!(event.value.effects, [None, Some(EffectKey(2))]);
}
#[test]
fn unchanged_hook_output_keeps_the_value() {
    let mut event = event();
    let original = event.clone();
    host::write(&mut event, wire::BoundaryEventOut { value: None }).unwrap();
    assert_eq!(event, original);
}
