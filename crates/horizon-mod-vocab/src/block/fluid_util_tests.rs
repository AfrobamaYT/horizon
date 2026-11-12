use crate::prelude::PlayersHost;
use crate::testing::{FakeGameEvent, FakeHost, FakeSound, FakeTank};
use crate::{
    FluidResource, FluidStack, GameEventContext, Hand, Identifier, ItemStack, ServerHost,
    SoundSource, fluid_util,
};

const TANK: [i32; 3] = [0, 64, 0];
const PLAYER: u128 = 7;
const PLAYER_ENTITY: u32 = 70;

/// A host with an 8000 mB tank at `TANK`, vanilla's buckets and fluids with their bucket sounds,
/// and player `PLAYER` holding `count` of `item` in the main hand.
fn host(item: &str, count: u32) -> FakeHost {
    let mut host = FakeHost::new();
    let items = [
        "minecraft:bucket",
        "minecraft:water_bucket",
        "minecraft:lava_bucket",
    ];
    host.registries
        .resolve_items(items.map(|id| Identifier::parse(id).unwrap()))
        .unwrap();
    host.registries
        .resolve_fluids(["minecraft:empty", "minecraft:water", "minecraft:lava"])
        .unwrap();
    host.registries
        .fluid_sounds(
            "minecraft:water",
            "minecraft:item.bucket.fill",
            "minecraft:item.bucket.empty",
        )
        .fluid_sounds(
            "minecraft:lava",
            "minecraft:item.bucket.fill_lava",
            "minecraft:item.bucket.empty_lava",
        );
    host.level.block_entity(TANK).tanks = vec![FakeTank::new(8000)];
    host.join(PLAYER, PLAYER_ENTITY)
        .slots
        .insert(0, stack(item, count));
    host
}

fn stack(item: &str, count: u32) -> ItemStack {
    let key = crate::__private::item_key(item).unwrap();
    ItemStack::new(key, count).unwrap()
}

fn fluid(id: &str, amount: u32) -> FluidStack {
    FluidStack::new(crate::__private::fluid_key(id).unwrap(), amount)
}

/// `PLAYER` right-clicks the tank with the main hand.
fn click(host: &mut FakeHost) -> bool {
    let server: &mut dyn ServerHost = host;
    let player = server.player(PLAYER).unwrap();
    let level = player.level(server).unwrap();
    fluid_util::interact_with_fluid_handler(server, player, Hand::Main, level, TANK, None)
}

fn held(host: &FakeHost) -> Option<FluidStack> {
    host.level.block_entity_at(TANK).unwrap().tanks[0]
        .held
        .clone()
}

fn hand(host: &FakeHost) -> Option<ItemStack> {
    host.players.get(PLAYER).unwrap().main_hand().cloned()
}

/// The bucket's sound and game event at the centre of the tank, the player its event's source.
fn effects(host: &FakeHost, sound: &str, event: &str) -> (Vec<FakeSound>, Vec<FakeGameEvent>) {
    let at = [0.5, 64.5, 0.5];
    let source = host.player(PLAYER).map(|player| player.entity());
    let sound = FakeSound {
        sound: sound.to_owned(),
        at,
        source: SoundSource::Blocks,
        volume: 1.0,
        pitch: 1.0,
    };
    let event = FakeGameEvent {
        event: event.to_owned(),
        at,
        context: GameEventContext {
            source_entity: source,
            affected_state: None,
        },
    };
    (vec![sound], vec![event])
}

#[test]
fn a_water_bucket_empties_into_an_empty_tank_with_the_empty_sound() {
    let mut host = host("minecraft:water_bucket", 1);
    assert!(click(&mut host));
    assert_eq!(held(&host), Some(fluid("minecraft:water", 1000)));
    assert_eq!(hand(&host), Some(stack("minecraft:bucket", 1)));
    let expected = effects(
        &host,
        "minecraft:item.bucket.empty",
        "minecraft:fluid_place",
    );
    assert_eq!((host.sounds.clone(), host.game_events.clone()), expected);
}

#[test]
fn an_empty_bucket_fills_from_the_tank_first_and_splits_off_its_stack() {
    let mut host = host("minecraft:bucket", 3);
    host.level.block_entity(TANK).tanks[0].held = Some(fluid("minecraft:lava", 2500));
    assert!(click(&mut host));
    assert_eq!(held(&host), Some(fluid("minecraft:lava", 1500)));
    assert_eq!(hand(&host), Some(stack("minecraft:bucket", 2)));
    let player = host.players.get(PLAYER).unwrap();
    assert_eq!(player.given, vec![stack("minecraft:lava_bucket", 1)]);
    let expected = effects(
        &host,
        "minecraft:item.bucket.fill_lava",
        "minecraft:fluid_pickup",
    );
    assert_eq!((host.sounds.clone(), host.game_events.clone()), expected);
}

#[test]
fn a_full_bucket_fills_no_further_and_empties_into_a_tank_of_its_fluid() {
    let mut host = host("minecraft:water_bucket", 1);
    host.level.block_entity(TANK).tanks[0].held = Some(fluid("minecraft:water", 7000));
    assert!(click(&mut host));
    assert_eq!(held(&host), Some(fluid("minecraft:water", 8000)));
    assert_eq!(hand(&host), Some(stack("minecraft:bucket", 1)));
    assert_eq!(host.game_events[0].event, "minecraft:fluid_place");
}

#[test]
fn nothing_moves_into_a_tank_of_another_fluid_or_one_without_room() {
    for (bucket, in_tank, amount) in [
        ("minecraft:lava_bucket", "minecraft:water", 1000),
        ("minecraft:water_bucket", "minecraft:water", 7500),
    ] {
        let mut host = host(bucket, 1);
        let tank = fluid(in_tank, amount);
        host.level.block_entity(TANK).tanks[0].held = Some(tank.clone());
        assert!(!click(&mut host), "{bucket}");
        assert_eq!(held(&host), Some(tank), "{bucket}");
        assert_eq!(hand(&host), Some(stack(bucket, 1)), "{bucket}");
        assert!(host.sounds.is_empty() && host.game_events.is_empty());
    }
}

#[test]
fn a_hand_without_a_fluid_container_or_a_block_without_tanks_does_nothing() {
    let mut host = host("minecraft:bucket", 1);
    host.players.get_mut(PLAYER).unwrap().slots.clear();
    assert!(!click(&mut host));
    let mut host = host_without_tank();
    assert!(!click(&mut host));
    assert_eq!(hand(&host), Some(stack("minecraft:water_bucket", 1)));
    assert!(host.sounds.is_empty() && host.game_events.is_empty());
}

fn host_without_tank() -> FakeHost {
    let mut host = host("minecraft:water_bucket", 1);
    host.level.block_entity(TANK).tanks.clear();
    host
}

#[test]
fn a_fluid_without_a_bucket_sound_sends_only_its_game_event() {
    let mut host = host("minecraft:bucket", 1);
    let server: &mut dyn ServerHost = &mut host;
    let level = server.level("minecraft:overworld").unwrap();
    // The fake gives `minecraft:empty` no sounds, as the game gives none to a tank-only fluid.
    let empty = FluidResource::of(&fluid("minecraft:empty", 1));
    fluid_util::trigger_sound_and_game_event(server, &empty, level, [1.0, 2.0, 3.0], None, true);
    assert!(host.sounds.is_empty());
    let event = FakeGameEvent {
        event: "minecraft:fluid_pickup".to_owned(),
        at: [1.0, 2.0, 3.0],
        context: GameEventContext::default(),
    };
    assert_eq!(host.game_events, vec![event]);
}
