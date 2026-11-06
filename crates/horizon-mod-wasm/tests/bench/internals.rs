//! A real guest crossing the internals imports, not just the native frame implementation.
use mod_points::internals::{
    self as I, Arg, Member, Op, Place, Refusal, Reply, Table, Thunk, Type, Value,
};

#[derive(Debug, PartialEq)]
struct Game {
    copy: i64,
    child: Child,
}
#[derive(Debug, PartialEq)]
struct Child {
    value: i64,
    padding: Vec<u8>,
}
struct Record {
    message: String,
    cancel: bool,
}

fn world() -> Game {
    Game {
        copy: 7,
        child: Child {
            value: 11,
            padding: Vec::new(),
        },
    }
}
unsafe fn game(ptr: *mut (), writable: bool, op: Op<'_>) -> Result<Reply, Refusal> {
    // SAFETY: this test's descriptor names exactly the live Game supplied to fire_raw.
    let game = unsafe { &mut *ptr.cast::<Game>() };
    match op {
        Op::Get(0) => Ok(Reply::Value(Value::S(game.copy))),
        Op::Set(0, Value::S(n)) if writable => {
            game.copy = n;
            Ok(Reply::Done)
        }
        Op::Field(1) => Ok(Reply::Node((&raw mut game.child).cast(), 1, writable)),
        _ => Err(I::not_here("not a benchmark game field")),
    }
}
unsafe fn child(ptr: *mut (), _: bool, op: Op<'_>) -> Result<Reply, Refusal> {
    // SAFETY: only the Game's child step supplies this node.
    let child = unsafe { &*ptr.cast::<Child>() };
    match op {
        Op::Get(0) => Ok(Reply::Value(Value::S(child.value))),
        _ => Err(I::not_here("not a benchmark child field")),
    }
}
unsafe fn record(ptr: *mut (), writable: bool, op: Op<'_>) -> Result<Reply, Refusal> {
    // SAFETY: the descriptor below names this exact raw record layout.
    let record = unsafe { &mut *ptr.cast::<Record>() };
    match op {
        Op::Get(0) => Ok(Reply::Value(Value::Str(record.message.clone()))),
        Op::Get(1) => Ok(Reply::Value(Value::B(record.cancel))),
        Op::Set(0, Value::Str(message)) if writable => {
            record.message = message;
            Ok(Reply::Done)
        }
        _ => Err(I::not_here("not a benchmark raw field")),
    }
}
#[inline(never)]
fn native_add(game: &mut Game, delta: i64) -> i64 {
    game.copy += delta;
    game.copy
}
unsafe fn add(receiver: Option<*mut ()>, args: Vec<Arg>) -> Result<Reply, Refusal> {
    // SAFETY: Frame checked the receiver's type and exclusive write access.
    let game = unsafe { &mut *receiver.unwrap().cast::<Game>() };
    let [arg] = I::fields::<_, 1>(args)?;
    Ok(Reply::Value(Value::S(native_add(
        game,
        I::int(I::value(arg)?)?,
    ))))
}

fn table() -> &'static Table {
    let catalog: serde_json::Value =
        serde_json::from_str(include_str!("internals/internals.json")).unwrap();
    let hash = |path: &str| -> &'static str {
        Box::leak(
            catalog
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["path"] == path)
                .unwrap()["hash"]
                .as_str()
                .unwrap()
                .to_owned()
                .into_boxed_str(),
        )
    };
    let point = mod_points::PointId::HandleChatInput as usize;
    Box::leak(Box::new(Table {
        types: Box::leak(Box::new([
            Type {
                path: "horizon_client::game::Game",
                hash: hash("horizon_client::game::Game"),
                node: game,
            },
            Type {
                path: "horizon_client::game::Child",
                hash: hash("horizon_client::game::Child"),
                node: child,
            },
            Type {
                path: "fixture::Raw",
                hash: "",
                node: record,
            },
        ])),
        members: Box::leak(Box::new([
            Member {
                path: "horizon_client::game::Game::copy",
                hash: hash("horizon_client::game::Game::copy"),
                owner: 0,
                field: 0,
            },
            Member {
                path: "horizon_client::game::Game::child",
                hash: hash("horizon_client::game::Game::child"),
                owner: 0,
                field: 1,
            },
        ])),
        thunks: Box::leak(Box::new([Thunk {
            path: "horizon_client::game::Game::add",
            hash: hash("horizon_client::game::Game::add"),
            receiver: Some(0),
            args: &[None],
            writes: 1,
            call: add,
        }])),
        places: Box::leak(Box::new([Place {
            place: point,
            target: mod_points::places::TARGETS[point],
            hash: mod_points::places::SHAPES[point],
            receiver: Some(0),
            receiver_writable: true,
            record: 2,
        }])),
        ..Table::EMPTY
    }))
}

fn wasm(mode: &str, count: i64) -> (Game, String) {
    let mut game = world();
    let mut record = Record {
        message: format!("{mode} {count}"),
        cancel: false,
    };
    mod_points::__private::fire_raw(
        mod_points::PointId::HandleChatInput,
        "handle_chat_input",
        &mut |kind| {
            let mod_points::__private::RawKind::Erased(callback) = kind else {
                panic!("wasm listener")
            };
            // SAFETY: table() supplies exactly these receiver and record layouts.
            unsafe {
                callback(
                    mod_points::PointId::HandleChatInput as usize,
                    (&raw mut game).cast(),
                    (&raw mut record).cast(),
                );
            }
        },
    );
    (game, record.message)
}
fn native(mode: &str, count: i64) -> (Game, String) {
    let mut game = world();
    let mut total = 0;
    for i in 0..std::hint::black_box(count) {
        let game = std::hint::black_box(&mut game);
        match mode {
            "get" => total += game.copy,
            "handle" => total += game.child.value,
            "set" => game.copy = i,
            "call" => total += native_add(game, 1),
            _ => unreachable!(),
        }
    }
    (game, total.to_string())
}

#[test]
#[ignore = "needs tests/bench/internals built to mod.wasm; run alone with --test-threads=1"]
fn internals_accesses_against_native() {
    horizon_mod_wasm::set_internals(table());
    let path = |name: &str| std::path::PathBuf::from(std::env::var(name).expect(name));
    let package = path("HORIZON_WASM_INTERNALS_BENCH");
    let loaded = horizon_mod_wasm::load(
        &horizon_mod_wasm::RuntimeConfig::default(),
        &std::fs::read(path("HORIZON_WASM_STD")).unwrap(),
        vec![horizon_mod_wasm::WasmPackage {
            manifest: horizon_mods::Package::read(&package).unwrap().manifest,
            module: horizon_mods::Package::read_wasm(&package).unwrap(),
        }],
    );
    assert!(loaded.refused.is_empty(), "{:?}", loaded.refused);
    for listeners in loaded.listening {
        listeners.register().unwrap();
    }
    mod_points::__private::seal();
    let mut report = String::from(
        "### Internals imports against native\n\nMedian of 9 batches after warm-up. Each call performs 256 accesses; the zero-access call is subtracted before dividing by 256. Set includes the journal apply at return. Handle read includes field then get. Native accesses are black-boxed per iteration.\n\n| Access | Wasm ns | Native ns |\n| --- | ---: | ---: |\n",
    );
    for mode in ["get", "handle", "set", "call"] {
        assert_eq!(
            wasm(mode, 256),
            native(mode, 256),
            "{mode}: committed values and result"
        );
        let measure = |call: fn(&str, i64) -> (Game, String)| {
            let base = super::timed(100, || {
                std::hint::black_box(call(mode, 0));
            });
            let work = super::timed(100, || {
                std::hint::black_box(call(mode, 256));
            });
            (work.as_secs_f64() - base.as_secs_f64()).max(0.0) * 1e9 / 256.0
        };
        report.push_str(&format!(
            "| {mode} | {:.2} | {:.2} |\n",
            measure(wasm),
            measure(native)
        ));
    }
    assert_eq!(
        wasm("trap", 1),
        (world(), "trap 1".into()),
        "a real guest trap discards its pending set"
    );
    assert_eq!(
        wasm("get", 1),
        (world(), "get 1".into()),
        "the trapped group stays stopped"
    );
    println!("{report}");
    if let Ok(path) = std::env::var("HORIZON_WASM_INTERNALS_BENCH_REPORT") {
        std::fs::write(path, report).unwrap();
    }
}
