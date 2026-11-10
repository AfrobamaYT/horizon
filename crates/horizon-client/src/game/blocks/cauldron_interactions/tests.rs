use super::*;
use crate::game::native_entity_refs::tests::with_game_at;
use mod_api::{CauldronDecision, CauldronFire, CauldronPlan, CauldronUse, Hooks, Host, ServerHost};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

const POS: [i32; 3] = [0, 64, 0];
static CHILDREN: Mutex<()> = Mutex::new(());

fn fresh(name: &str) -> bool {
    if std::env::var("HORIZON_CAULDRON_CASE").as_deref() == Ok(name) {
        return false;
    }
    let _serial = CHILDREN.lock().unwrap_or_else(|e| e.into_inner());
    let scratch = tempfile::tempdir().unwrap();
    let resources = std::path::PathBuf::from(std::env::var_os("HORIZON_TEST_RESOURCES").unwrap())
        .canonicalize()
        .unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            &format!("{}::{name}", module_path!().split_once("::").unwrap().1),
            "--nocapture",
        ])
        .current_dir(scratch.path())
        .env("HORIZON_CAULDRON_CASE", name)
        .env("HORIZON_TEST_RESOURCES", resources)
        .env("HORIZON_NO_AUDIO", "1")
        .env("HORIZON_HOME", scratch.path().join("home"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "child matched no test: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    true
}
#[derive(Default)]
struct Seen {
    decide: AtomicUsize,
    actor: AtomicUsize,
    fire: AtomicUsize,
    drops: AtomicUsize,
}
struct Syrup {
    seen: Arc<Seen>,
    refused: bool,
    panic: bool,
}
impl mod_api::Interaction for Syrup {
    fn decide(&self, host: &dyn Host, input: &CauldronUse) -> CauldronDecision {
        self.seen.decide.fetch_add(1, Ordering::SeqCst);
        self.seen
            .actor
            .store(input.actor.id() as usize, Ordering::SeqCst);
        if input.actor.id() == 7 {
            assert_eq!(host.entity(7), Some(input.actor));
        }
        CauldronDecision::Success(CauldronPlan {
            block: Some(host.block_state_id("test:syrup_cauldron[level=1]").unwrap()),
            exchange: Some(
                mod_api::ItemStack::new(
                    mod_api::__private::item_key("minecraft:glass_bottle").unwrap(),
                    1,
                )
                .unwrap(),
            ),
            sound: Some(mod_api::Identifier::parse("minecraft:item.bottle.empty").unwrap()),
            fluid_event: Some(mod_api::Identifier::parse("minecraft:fluid_place").unwrap()),
            statistics: vec![
                mod_api::Identifier::parse("minecraft:use_cauldron").unwrap(),
                mod_api::Identifier::parse("minecraft:used").unwrap(),
            ],
        })
    }
    fn fire(&self, _: &mut dyn ServerHost, _: &CauldronUse, _: &CauldronPlan) -> CauldronFire {
        self.seen.fire.fetch_add(1, Ordering::SeqCst);
        assert!(!self.panic, "fixture callback panic");
        if self.refused {
            CauldronFire::Refused
        } else {
            CauldronFire::Applied
        }
    }
}
struct TestMod {
    seen: Arc<Seen>,
    refused: bool,
    panic: bool,
    dye: bool,
}
#[mod_api::__subscriptions]
impl Hooks for TestMod {
    fn blocks(&self, _: &mod_api::RegistrationContext, event: &mut mod_api::BlocksEvent<'_>) {
        event.blocks.push(mod_api::Block::new(
            "test:syrup_cauldron",
            "minecraft:water_cauldron",
        ));
    }
    fn items(&self, _: &mod_api::RegistrationContext, event: &mut mod_api::ItemsEvent<'_>) {
        event.items.push(mod_api::Item::new(
            "test:syrup_bottle",
            "minecraft:glass_bottle",
        ));
    }
    fn register_cauldron_interaction(
        &self,
        _: &mod_api::RegistrationContext,
        event: &mut mod_api::RegisterCauldronInteractionEvent<'_>,
    ) {
        let handler: Arc<dyn mod_api::Interaction> = Arc::new(Syrup {
            seen: self.seen.clone(),
            refused: self.refused,
            panic: self.panic,
        });
        let item = mod_api::__private::item_key(if self.dye {
            "minecraft:leather_helmet"
        } else {
            "test:syrup_bottle"
        })
        .unwrap();
        let block = mod_api::__private::block_key(if self.dye {
            "minecraft:water_cauldron"
        } else {
            "minecraft:cauldron"
        })
        .unwrap();
        event
            .interactions
            .register(block, item, handler.clone())
            .unwrap();
        event
            .interactions
            .register(
                mod_api::__private::block_key("test:syrup_cauldron").unwrap(),
                mod_api::__private::item_key("test:syrup_bottle").unwrap(),
                handler,
            )
            .unwrap();
    }
    fn item_spawn(&self, _: &mut dyn ServerHost, _: &mut mod_api::ItemSpawnEvent<'_>) {
        self.seen.drops.fetch_add(1, Ordering::SeqCst);
    }
}
fn fixture(refused: bool, panic: bool, dye: bool, test: impl FnOnce(&mut Game<'_>, &Arc<Seen>)) {
    let seen = Arc::new(Seen::default());
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
        "test",
        Box::new(TestMod {
            seen: seen.clone(),
            refused,
            panic,
            dye,
        }),
    )]
    .into();
    mod_api::__private::install(mods.clone());
    crate::game::api_values::install_parts(&mods).unwrap();
    let base = std::path::PathBuf::from(std::env::var_os("HORIZON_TEST_RESOURCES").unwrap());
    let root = crate::game::mods::content_root(&base, &Default::default())
        .unwrap()
        .prepared()
        .to_path_buf();
    with_game_at(root, |game| {
        setup(game, "minecraft:cauldron", None, "test:syrup_bottle", 2);
        test(game, &seen);
    });
}
fn setup(game: &mut Game<'_>, block: &str, level: Option<&str>, item: &str, count: u32) {
    game.world
        .chunks
        .insert((0, 0), Arc::new(horizon_client::chunk::Chunk::empty(0, 0)));
    let state = horizon_world::chunk::BlockState {
        name: block.into(),
        properties: level
            .map(|v| ("level".into(), v.into()))
            .into_iter()
            .collect(),
    };
    let id = game.pack.assets.registry.resolve(&state).unwrap();
    game.world.set(0, 64, 0, id).unwrap();
    game.player = Some(
        horizon_client::motion::PlayerMotion::new(game.motion, horizon_core::geometry::Vec3::ZERO)
            .unwrap(),
    );
    game.creative = false;
    game.selected = 0;
    game.hand = Hand::Main;
    game.inventory.fill(None);
    game.inventory[0] = Some(ItemStack::new(item, count));
    game.audio.capture = Some(Vec::new());
    crate::game::sculk::TEST_POSTED_EVENTS.with(|events| *events.borrow_mut() = Some(Vec::new()));
}
fn state(game: &Game<'_>) -> (String, Option<String>) {
    let state = game
        .pack
        .assets
        .registry
        .state(game.world.block(0, 64, 0))
        .unwrap();
    (state.name.clone(), state.properties.get("level").cloned())
}
fn events() -> usize {
    crate::game::sculk::TEST_POSTED_EVENTS.with(|events| events.borrow().as_ref().unwrap().len())
}
fn unchanged(game: &Game<'_>, seen: &Seen) {
    assert_eq!(state(game), ("minecraft:cauldron".into(), None));
    let held = game.inventory[0].as_ref().unwrap();
    assert_eq!((&*held.item, held.count), ("test:syrup_bottle", 2));
    assert_eq!(game.audio.capture.as_ref().unwrap().len(), 0);
    assert_eq!(events(), 0);
    assert_eq!(seen.drops.load(Ordering::SeqCst), 0);
    assert_eq!(game.dropped.len(), 0);
    assert_eq!(
        game.inventory
            .iter()
            .flatten()
            .filter(|stack| &*stack.item == "minecraft:glass_bottle")
            .count(),
        0
    );
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn cauldron_syrup_applies_once() {
    if fresh("cauldron_syrup_applies_once") {
        return;
    }
    fixture(false, false, false, |game, seen| {
        assert!(game.use_on_cauldron(POS).unwrap());
        assert_eq!(seen.decide.load(Ordering::SeqCst), 1);
        assert_eq!(seen.fire.load(Ordering::SeqCst), 1);
        assert_eq!(game.inventory[0].as_ref().unwrap().count, 1);
        assert_eq!(
            state(game),
            ("test:syrup_cauldron".into(), Some("1".into()))
        );
        assert_eq!(
            game.inventory
                .iter()
                .flatten()
                .filter(|s| &*s.item == "minecraft:glass_bottle")
                .map(|s| s.count)
                .sum::<u32>(),
            1
        );
        let sounds = game.audio.capture.as_ref().unwrap();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].event, "minecraft:item.bottle.empty");
        assert_eq!(events(), 1);
        crate::game::sculk::TEST_POSTED_EVENTS.with(|events| {
            assert_eq!(
                events.borrow().as_ref().unwrap()[0].0,
                "minecraft:fluid_place"
            )
        });
        assert_eq!(seen.drops.load(Ordering::SeqCst), 0);
    });
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn cauldron_refused_fire_is_atomic() {
    if fresh("cauldron_refused_fire_is_atomic") {
        return;
    }
    fixture(true, false, false, |game, seen| {
        assert!(game.use_on_cauldron(POS).unwrap());
        unchanged(game, seen);
        assert_eq!(seen.fire.load(Ordering::SeqCst), 1);
    });
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn cauldron_panicking_fire_is_atomic() {
    if fresh("cauldron_panicking_fire_is_atomic") {
        return;
    }
    fixture(false, true, false, |game, seen| {
        assert!(game.use_on_cauldron(POS).unwrap());
        unchanged(game, seen);
        setup(
            game,
            "minecraft:cauldron",
            None,
            "minecraft:water_bucket",
            1,
        );
        assert!(game.use_on_cauldron(POS).unwrap());
        assert_eq!(
            state(game),
            ("minecraft:water_cauldron".into(), Some("3".into()))
        );
        assert_eq!(
            &*game.inventory[0].as_ref().unwrap().item,
            "minecraft:bucket"
        );
    });
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn cauldron_outer_abort_discards_effects_and_drops() {
    if fresh("cauldron_outer_abort_discards_effects_and_drops") {
        return;
    }
    fixture(false, false, false, |game, seen| {
        for slot in game.inventory.iter_mut().skip(1) {
            *slot = Some(ItemStack::new("minecraft:stone", 64));
        }
        let seal = game.transaction_open();
        assert!(game.use_on_cauldron(POS).unwrap());
        assert_eq!(events(), 0);
        assert_eq!(game.audio.capture.as_ref().unwrap().len(), 0);
        game.transaction_close(&seal, false);
        unchanged(game, seen);
        assert_eq!(game.stats.get("custom", "minecraft:use_cauldron"), 0);
        assert_eq!(game.stats.get("used", "test:syrup_bottle"), 0);
    });
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn cauldron_duplicates_name_both_owners() {
    if fresh("cauldron_duplicates_name_both_owners") {
        return;
    }
    fixture(false, false, false, |game, seen| {
        let mut registrar = Registrar {
            table: Table::default(),
            items: game.items(),
            registry: game.pack.assets.registry,
            all: Vec::new(),
            error: None,
        };
        let handler: Arc<dyn mod_api::Interaction> = Arc::new(Syrup {
            seen: seen.clone(),
            refused: false,
            panic: false,
        });
        use mod_api::CauldronInteractions as _;
        let block = mod_api::__private::block_key("minecraft:cauldron").unwrap();
        let item = mod_api::__private::item_key("test:syrup_bottle").unwrap();
        {
            let _owner = mod_api::__private::enter("first", "register_cauldron_interaction");
            assert!(registrar.register(block, item, handler.clone()).is_ok());
        }
        let _owner = mod_api::__private::enter("second", "register_cauldron_interaction");
        assert_eq!(
            registrar.register(block, item, handler.clone()),
            Err(mod_api::CauldronRegistrationError::Duplicate {
                owners: ["first".into(), "second".into()]
            })
        );
        assert_eq!(
            registrar.register(
                block,
                mod_api::__private::item_key("minecraft:water_bucket").unwrap(),
                handler
            ),
            Err(mod_api::CauldronRegistrationError::Duplicate {
                owners: ["minecraft".into(), "second".into()]
            })
        );
    });
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn cauldron_dye_tag_shadows_exact_handler_even_on_pass() {
    if fresh("cauldron_dye_tag_shadows_exact_handler_even_on_pass") {
        return;
    }
    fixture(false, false, true, |game, seen| {
        setup(
            game,
            "minecraft:water_cauldron",
            Some("2"),
            "minecraft:leather_helmet",
            1,
        );
        game.inventory[0].as_mut().unwrap().data_mut().dyed_color = Some(0x123456);
        assert!(game.use_on_cauldron(POS).unwrap());
        assert_eq!(
            state(game),
            ("minecraft:water_cauldron".into(), Some("1".into()))
        );
        assert_eq!(
            game.inventory[0]
                .as_ref()
                .unwrap()
                .data()
                .and_then(|d| d.dyed_color),
            None
        );
        assert!(!game.use_on_cauldron(POS).unwrap());
        assert_eq!(seen.decide.load(Ordering::SeqCst), 0);
        assert_eq!(seen.fire.load(Ordering::SeqCst), 0);
    });
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn cauldron_transfer_awards_two_statistics_once() {
    if fresh("cauldron_transfer_awards_two_statistics_once") {
        return;
    }
    fixture(false, false, false, |game, _| {
        assert!(game.use_on_cauldron(POS).unwrap());
        assert_eq!(game.stats.get("custom", "minecraft:use_cauldron"), 1);
        assert_eq!(game.stats.get("used", "test:syrup_bottle"), 1);
    });
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn cauldron_client_decides_without_firing_or_authoritative_writes() {
    if fresh("cauldron_client_decides_without_firing_or_authoritative_writes") {
        return;
    }
    fixture(false, false, false, |game, seen| {
        game.options.execution_role = crate::game::execution_role::ExecutionRole::Client;
        let _server = crate::game::native_client_session::tests::logged_client(game);
        setup(game, "minecraft:cauldron", None, "test:syrup_bottle", 2);
        let held = game.inventory[0].clone();
        let (outcome, proposed) = game.client_cauldron_prediction(POS, held.as_ref());
        assert_eq!(outcome, horizon_client::cauldron::Outcome::Success);
        assert!(proposed.0.is_empty());
        assert!(proposed.1.is_empty());
        assert_eq!(seen.decide.load(Ordering::SeqCst), 1);
        assert_eq!(seen.fire.load(Ordering::SeqCst), 0);
        assert_eq!(
            game.cauldron_outcome(POS, held.as_ref()),
            horizon_client::cauldron::Outcome::Success
        );
        assert_eq!(seen.decide.load(Ordering::SeqCst), 2);
        let hit = Hit {
            pos: POS,
            face: [0, 1, 0],
            point: [0.5, 65.0, 0.5],
            inside: false,
        };
        let (answer, _) = game.remote_cauldron_reacts(&hit, held.as_ref()).unwrap();
        assert!(answer.consumes_action());
        assert_eq!(seen.decide.load(Ordering::SeqCst), 3);
        assert_eq!(seen.actor.load(Ordering::SeqCst), 7);
        assert_eq!(seen.fire.load(Ordering::SeqCst), 0);
        unchanged(game, seen);
    });
}
