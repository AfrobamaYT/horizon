//! A real compiled guest view, loaded through the same package and erased raw route as the game.
use mod_points::internals::{
    self as I, Arg, Member, Op, Place, Refusal, Reply, Table, Thunk, Type, Value,
};

#[derive(Debug, PartialEq)]
struct Game {
    left: i64,
    right: i64,
    hidden: Vec<i32>,
}
struct Record {
    message: String,
    cancel: bool,
}

unsafe fn game(ptr: *mut (), writable: bool, op: Op<'_>) -> Result<Reply, Refusal> {
    // SAFETY: the synthetic point below supplies the live, exclusively borrowed fixture Game.
    let game = unsafe { &mut *ptr.cast::<Game>() };
    match op {
        Op::Get(0) => Ok(Reply::Value(Value::S(game.left))),
        Op::Get(1) => Ok(Reply::Value(Value::S(game.right))),
        Op::Set(0, Value::S(n)) if writable => {
            game.left = n;
            Ok(Reply::Done)
        }
        Op::Set(1, Value::S(n)) if writable => {
            game.right = n;
            Ok(Reply::Done)
        }
        _ => Err(I::not_here("not a declared game field")),
    }
}
unsafe fn record(ptr: *mut (), writable: bool, op: Op<'_>) -> Result<Reply, Refusal> {
    // SAFETY: the synthetic point supplies the live Record for this call only.
    let record = unsafe { &mut *ptr.cast::<Record>() };
    match op {
        Op::Get(0) => Ok(Reply::Value(Value::Str(record.message.clone()))),
        Op::Get(1) => Ok(Reply::Value(Value::B(record.cancel))),
        Op::Set(0, Value::Str(s)) if writable => {
            record.message = s;
            Ok(Reply::Done)
        }
        Op::Set(1, Value::B(b)) if writable => {
            record.cancel = b;
            Ok(Reply::Done)
        }
        _ => Err(I::not_here("not on the raw record")),
    }
}
unsafe fn add(receiver: Option<*mut ()>, args: Vec<Arg>) -> Result<Reply, Refusal> {
    // SAFETY: the checked thunk descriptor requires a writable Game receiver.
    let game = unsafe { &mut *receiver.unwrap().cast::<Game>() };
    let [arg] = I::fields::<_, 1>(args)?;
    let delta: i64 = I::int(I::value(arg)?)?;
    game.left += delta;
    Ok(Reply::Value(Value::S(game.left + game.right)))
}

#[test]
#[ignore = "needs tests/internals/counter built to mod.wasm (CI's wasm-mods job)"]
fn a_generated_guest_view_reads_writes_calls_and_commits_a_raw_listener() {
    let point = mod_points::PointId::HandleChatInput;
    let place = point as usize;
    let table = Box::leak(Box::new(Table {
        types: &[
            Type {
                path: "horizon_client::game::Game",
                hash: "6957fb66139038f6ab07ec4121dc6253b9f31532f1f3e45450b5439e695da605",
                node: game,
            },
            Type {
                path: "fixture::Raw",
                hash: "",
                node: record,
            },
        ],
        members: &[
            Member {
                path: "horizon_client::game::Game::left",
                hash: "71b38399f108e572a5ef928b5cfdf52872f61b4615115d91f588221900ab97e0",
                owner: 0,
                field: 0,
            },
            Member {
                path: "horizon_client::game::Game::right",
                hash: "7b6f34593a32100b46a93d4562b17b8adeb9c7e5645fe9f8efbccc519aadfcc2",
                owner: 0,
                field: 1,
            },
        ],
        thunks: &[Thunk {
            path: "horizon_client::game::Game::add",
            hash: "638665cf01d552d5765711831232b653f7b98d9c7bdddb54a58182e433e5cd0e",
            receiver: Some(0),
            args: &[None],
            writes: 1,
            call: add,
        }],
        places: Box::leak(Box::new([Place {
            place,
            target: mod_points::places::TARGETS[place],
            hash: mod_points::places::SHAPES[place],
            receiver: Some(0),
            receiver_writable: true,
            record: 1,
        }])),
        ..Table::EMPTY
    }));
    horizon_mod_wasm::set_internals(table);
    let path = |var: &str| std::path::PathBuf::from(std::env::var(var).expect(var));
    let package =
        path("HORIZON_WASM_INTERNALS").join("counter/target/horizon-mod/internals_counter.hmod");
    let loaded = horizon_mod_wasm::load(
        &horizon_mod_wasm::RuntimeConfig::default(),
        &std::fs::read(path("HORIZON_WASM_STD")).unwrap(),
        vec![horizon_mod_wasm::WasmPackage {
            manifest: horizon_mods::Package::read(&package).unwrap().manifest,
            module: horizon_mods::Package::read_wasm(&package).unwrap(),
        }],
    );
    assert!(loaded.refused.is_empty(), "{:?}", loaded.refused);
    for listening in loaded.listening {
        listening.register().unwrap();
    }
    mod_points::__private::seal();
    let mut game = Game {
        left: 1,
        right: 10,
        hidden: vec![99],
    };
    let mut raw = Record {
        message: "before".into(),
        cancel: false,
    };
    mod_points::__private::fire_raw(point, "handle_chat_input", &mut |kind| {
        let mod_points::__private::RawKind::Erased(callback) = kind else {
            panic!("a wasm raw listener")
        };
        // SAFETY: the descriptor above describes exactly these two live fixture roots.
        unsafe {
            callback(place, (&raw mut game).cast(), (&raw mut raw).cast());
        }
    });
    assert_eq!(
        game,
        Game {
            left: 8,
            right: 13,
            hidden: vec![99]
        }
    );
    assert_eq!(raw.message, "proof21");
    assert!(raw.cancel);
}
