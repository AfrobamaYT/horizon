use mod_api::{Hooks, ServerHost};
use serde_json::json;
use std::{
    fs::File,
    io::Write,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

static TICK: AtomicU64 = AtomicU64::new(0);
static OUTPUT: OnceLock<Mutex<File>> = OnceLock::new();

fn emit(name: &str, fields: serde_json::Value) {
    let output = OUTPUT.get_or_init(|| {
        Mutex::new(
            File::create(std::env::var_os("HORIZON_PAPER_TRACE").expect("HORIZON_PAPER_TRACE"))
                .expect("create trace"),
        )
    });
    let mut file = output.lock().unwrap();
    writeln!(
        file,
        "{}",
        json!({"tick": TICK.load(Ordering::Relaxed), "name": name,
        "fields": fields, "cancelled": false})
    )
    .expect("write trace");
    file.flush().expect("flush trace");
}

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn server_tick(&self, _: &mut dyn ServerHost, e: &mut mod_api::ServerTickEvent<'_>) {
        TICK.store(e.context.tick.get(), Ordering::Relaxed);
    }
    fn chunk_load(&self, _: &mut dyn ServerHost, e: &mut mod_api::ChunkLoadEvent<'_>) {
        emit(
            "chunk_load",
            json!({"dimension": e.dimension, "chunk": e.chunk, "new": e.new}),
        );
    }
    fn player_logged_in(&self, _: &mut dyn ServerHost, _: &mut mod_api::PlayerLoggedInEvent<'_>) {
        emit("_join", json!({}));
    }
    fn player_logged_out(&self, _: &mut dyn ServerHost, _: &mut mod_api::PlayerLoggedOutEvent<'_>) {
        emit("_quit", json!({}));
    }
}
