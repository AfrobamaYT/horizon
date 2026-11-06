//! Real-Game holder-boundary regressions and the ignored 1,000-ticker measurement.
use std::{sync::Arc, time::Instant};

use horizon_client::{
    chunk::Chunk,
    containers::{BlockEntity, ModBlockEntity},
};
use mod_api::{
    BlockEntityPart, BlockEntityTickEvent, BlockPart, Hooks, HostTransaction, Registrar, ServerHost,
};

struct Ticker(bool);
#[mod_api::horizon_part]
impl BlockPart for Ticker {}
#[mod_api::horizon_part]
impl BlockEntityPart for Ticker {
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        if self.0 {
            let level = host.level("minecraft:overworld").unwrap();
            let sink = host.block_entity(level, [0, 63, 0]).unwrap();
            let value = sink.data_slots(host).unwrap()[0];
            let result: Result<(), ()> = host.transaction(|host, _| {
                assert!(sink.set_data_slot(host, 0, value + 1));
                Ok(())
            });
            result.unwrap();
        }
        std::hint::black_box(event.pos);
    }
}
struct BenchMod(&'static str);
#[mod_api::__subscriptions]
impl Hooks for BenchMod {
    fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
        let kind = r.block_entity_type("ticker").blocks(&[]).build();
        if self.0 != "none" {
            r.block_entity_part(&kind, Ticker(self.0 == "write"))
                .unwrap();
        }
    }
}

#[test]
#[ignore = "measurement; needs HORIZON_TEST_RESOURCES and HORIZON_LIVE_BENCH=none|trivial|write"]
fn thousand_live_block_entity_ticks() {
    let mode = std::env::var("HORIZON_LIVE_BENCH").unwrap();
    let mode = match mode.as_str() {
        "none" => "none",
        "trivial" => "trivial",
        "write" => "write",
        _ => panic!("unknown measurement mode"),
    };
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
        "live_bench",
        Box::new(BenchMod(mode)),
    )]
    .into();
    mod_api::testing::with_mods(mods.clone(), || {
        mod_api::__private::install_parts(&mods).unwrap();
        mod_api::__private::mark_game_thread();
        let base = std::path::PathBuf::from(std::env::var_os("HORIZON_TEST_RESOURCES").unwrap());
        let root = super::mods::content_root(&base, &Default::default()).unwrap();
        super::native_entity_refs::tests::with_game_at(root, |game| {
            for x in 0..4 {
                for z in 0..4 {
                    game.world
                        .chunks
                        .insert((x, z), Arc::new(Chunk::empty(x, z)));
                }
            }
            let positions: Vec<_> = (0..1000).map(|i| (i % 40, 64, i / 40)).collect();
            for &pos in positions.iter().chain(std::iter::once(&(0, 63, 0))) {
                let mut entity =
                    ModBlockEntity::new("live_bench:ticker".into(), Default::default());
                entity.data = vec![0];
                game.world
                    .block_entities
                    .insert(pos, BlockEntity::Mod(Box::new(entity)));
            }
            let mut times = Vec::with_capacity(240);
            for round in 0..260 {
                let start = Instant::now();
                for &pos in &positions {
                    game.tick_mod_block_entity(pos);
                }
                let elapsed = start.elapsed().as_nanos();
                if round >= 20 {
                    times.push(elapsed);
                }
            }
            times.sort_unstable();
            if mode == "write" {
                let BlockEntity::Mod(sink) = game.world.block_entities.get(&(0, 63, 0)).unwrap()
                else {
                    panic!("sink removed")
                };
                assert_eq!(sink.data, [260_000]);
            }
            println!(
                "LIVE_BENCH mode={mode} holders=1000 samples={} median_ns={} p95_ns={} p99_ns={}",
                times.len(),
                times[120],
                times[228],
                times[237]
            );
        });
    });
}
