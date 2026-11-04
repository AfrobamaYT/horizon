//! What a wasm mod costs against the same mod built native: `tests/bench/` runs one chat hook
//! doing a chosen amount of work, compiled into this test and loaded as `mod.wasm`. Each case
//! reports the median time per call over batches, and the test checks both builds answered
//! alike. CI builds the mod first, points `HORIZON_WASM_BENCH` at its `.hmod` and
//! `HORIZON_WASM_STD` at the std member, runs this in release and adds the table to the job
//! summary; the test is ignored elsewhere.
use std::time::{Duration, Instant};

use mod_api::__private::game::Fields;
use mod_api::{Hooks, Host, LevelRef, NativeWorldEpoch, ServerChatEvent, ServerHost, Text};

#[path = "bench/src/lib.rs"]
mod bench_mod;

/// One overworld, and nothing else.
struct World;

impl Host for World {
    fn level(&self, dimension: &str) -> Option<LevelRef> {
        (dimension == "minecraft:overworld")
            .then(|| LevelRef::new(NativeWorldEpoch::new(1), "minecraft:overworld"))
    }
}

impl ServerHost for World {}

/// One call of the chat hook with `work` as the message; what the mod wrote back.
fn chat(hooks: &dyn Hooks, work: &str) -> String {
    let mut message = Text::literal("");
    let mut replies = Vec::new();
    let mut event = ServerChatEvent {
        player_id: 1,
        player: 1,
        username: "bench",
        raw_text: work,
        message: &mut message,
        replies: &mut replies,
    };
    hooks.server_chat(&mut World, &mut event);
    message.plain()
}

/// The median time per call of `calls` calls, over 9 batches after one warm-up batch.
fn per_call(hooks: &dyn Hooks, work: &str, calls: u32) -> Duration {
    let mut batches: Vec<Duration> = (0..10)
        .map(|_| {
            let started = Instant::now();
            for _ in 0..calls {
                std::hint::black_box(chat(hooks, std::hint::black_box(work)));
            }
            started.elapsed() / calls
        })
        .skip(1)
        .collect();
    batches.sort();
    batches[batches.len() / 2]
}

fn shown(time: Duration) -> String {
    match time.as_nanos() {
        n if n < 10_000 => format!("{n} ns"),
        n if n < 10_000_000 => format!("{:.1} µs", n as f64 / 1e3),
        n => format!("{:.1} ms", n as f64 / 1e6),
    }
}

#[test]
#[ignore = "needs tests/bench built to mod.wasm and a release build (CI's wasm-bench job)"]
fn a_wasm_mod_against_the_same_mod_native() {
    let path = |var: &str| std::path::PathBuf::from(std::env::var(var).expect(var));
    let package = path("HORIZON_WASM_BENCH");
    let std_member = std::fs::read(path("HORIZON_WASM_STD")).unwrap();
    let loaded = horizon_mod_wasm::load(
        &horizon_mod_wasm::RuntimeConfig::default(),
        &std_member,
        vec![horizon_mod_wasm::WasmPackage {
            manifest: horizon_mods::Package::read(&package).unwrap().manifest,
            module: horizon_mods::Package::read_wasm(&package).unwrap(),
        }],
    );
    assert!(loaded.refused.is_empty(), "{:?}", loaded.refused);
    let wasm = loaded.mods[0].fields().hooks;
    let native = &*bench_mod::hooks();

    let cases = [
        ("empty", "the call alone", 20_000),
        ("text", "a message lowered back", 20_000),
        ("imports 100", "100 calls into the game", 1_000),
        ("compute 1000", "logic on 1,000 values", 1_000),
        ("compute 10000", "logic on 10,000 values", 100),
    ];
    let mut table = String::from(
        "### A wasm mod against the same mod native\n\n\
         | Work | | Native | Wasm | Ratio |\n| --- | --- | --- | --- | --- |\n",
    );
    for (work, what, calls) in cases {
        assert_eq!(
            chat(native, work),
            chat(wasm, work),
            "{work}: both builds answer alike"
        );
        let native_time = per_call(native, work, calls);
        let wasm_time = per_call(wasm, work, calls);
        table.push_str(&format!(
            "| `{work}` | {what} | {} | {} | {:.1}× |\n",
            shown(native_time),
            shown(wasm_time),
            wasm_time.as_secs_f64() / native_time.as_secs_f64().max(1e-12)
        ));
    }
    println!("{table}");
    if let Ok(summary) = std::env::var("GITHUB_STEP_SUMMARY") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(summary)
            .unwrap();
        writeln!(file, "{table}").unwrap();
    }
}
