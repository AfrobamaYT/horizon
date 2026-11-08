use super::*;
use crate::game::{
    commands::Target,
    native_entity_refs::tests::{with_client_game, with_game},
};
use horizon_core::geometry::Vec3;
use mod_api::{EntityEnteringSectionEvent, Hooks, HostSide};
use std::sync::{Arc, Mutex};

/// What each told crossing carried: client side, entity id, old, new, chunk changed.
type Calls = Arc<Mutex<Vec<(bool, u32, [i32; 3], [i32; 3], bool)>>>;

struct Listener(Calls);

#[mod_api::__subscriptions]
impl Hooks for Listener {
    fn entity_entering_section(&self, host: HostSide<'_>, event: &mut EntityEnteringSectionEvent) {
        self.0.lock().unwrap().push((
            matches!(host, HostSide::Client(_)),
            event.entity.id(),
            event.old_section,
            event.new_section,
            event.chunk_changed,
        ));
    }
}

fn observe(test: impl FnOnce(Calls)) {
    let calls = Calls::default();
    let mods = vec![mod_api::ModEntry::new(
        "sections",
        Box::new(Listener(calls.clone())),
    )]
    .into();
    mod_api::testing::with_mods(mods, || test(calls));
}

/// One look at `entities` (id, position): its crossings.
fn look(sections: &mut EntitySections, entities: &[(u32, [f64; 3])]) -> Vec<Crossing> {
    sections.begin(Some(NativeWorldEpoch::new(1)));
    for &(id, at) in entities {
        sections.see(id, at);
    }
    sections.end();
    sections.crossed.clone()
}

fn crossing(id: u32, old: [i32; 3], new: [i32; 3]) -> Crossing {
    Crossing { id, old, new }
}

#[test]
fn a_section_is_the_floored_block_shifted_by_four() {
    // Blocks 0..=15 are section 0, -1..=-16 section -1, -17 section -2.
    assert_eq!(section_of([0.0, 15.999, 15.0]), [0, 0, 0]);
    assert_eq!(section_of([16.0, 64.0, 31.5]), [1, 4, 1]);
    assert_eq!(section_of([-0.001, -16.0, -16.001]), [-1, -1, -2]);
    // Java's cast saturates and NaN is 0; `Mth.floor` then wraps `MIN - 1` to `MAX`.
    assert_eq!(section_coord(f64::NAN), 0);
    assert_eq!(section_coord(1e300), i32::MAX >> 4);
    assert_eq!(section_coord(f64::from(i32::MIN)), -134_217_728);
    assert_eq!(section_coord(f64::from(i32::MIN) - 0.5), 134_217_727);
    assert_eq!(section_coord(f64::NEG_INFINITY), 134_217_727);
}

#[test]
fn walking_over_an_edge_is_one_crossing_and_staying_inside_is_none() {
    let mut sections = EntitySections::default();
    // First sight: the entity joins its section untold.
    assert!(look(&mut sections, &[(5, [15.5, 70.0, 3.0])]).is_empty());
    // Moving about inside the section.
    assert!(look(&mut sections, &[(5, [1.0, 64.0, 15.9])]).is_empty());
    // East over x = 16.
    assert_eq!(
        look(&mut sections, &[(5, [16.2, 64.0, 15.9])]),
        [crossing(5, [0, 4, 0], [1, 4, 0])]
    );
    // Told once: standing in the new section stays quiet.
    assert!(look(&mut sections, &[(5, [16.2, 64.0, 15.9])]).is_empty());
    // Down over y = 64, then north into negative z.
    assert_eq!(
        look(&mut sections, &[(5, [16.2, 63.9, 15.9])]),
        [crossing(5, [1, 4, 0], [1, 3, 0])]
    );
    assert_eq!(
        look(&mut sections, &[(5, [16.2, 63.9, -0.1])]),
        [crossing(5, [1, 3, 0], [1, 3, -1])]
    );
}

#[test]
fn a_teleport_is_one_crossing_from_where_it_was() {
    let mut sections = EntitySections::default();
    look(&mut sections, &[(9, [8.0, 70.0, 8.0])]);
    // Many sections away at once, in every axis.
    assert_eq!(
        look(&mut sections, &[(9, [-1000.0, -40.0, 5000.0])]),
        [crossing(9, [0, 4, 0], [-63, -3, 312])]
    );
}

#[test]
fn several_entities_crossing_in_one_tick_are_each_told_once() {
    let mut sections = EntitySections::default();
    look(
        &mut sections,
        &[
            (1, [0.5, 70.0, 0.5]),
            (2, [-0.5, 70.0, 0.5]),
            (3, [40.0, 0.0, 40.0]),
        ],
    );
    assert_eq!(
        look(
            &mut sections,
            &[
                (1, [-0.5, 70.0, 0.5]),
                (2, [0.5, 70.0, 0.5]),
                (3, [41.0, 0.0, 40.0])
            ],
        ),
        [
            crossing(1, [0, 4, 0], [-1, 4, 0]),
            crossing(2, [-1, 4, 0], [0, 4, 0]),
        ]
    );
}

#[test]
fn an_entity_that_left_comes_back_untold_and_a_new_world_starts_over() {
    let mut sections = EntitySections::default();
    look(&mut sections, &[(4, [0.5, 70.0, 0.5])]);
    // Gone for a look (it left the level): back elsewhere, it joins untold.
    assert!(look(&mut sections, &[]).is_empty());
    assert!(look(&mut sections, &[(4, [100.0, 70.0, 0.5])]).is_empty());
    // Forgotten (a respawn): untold too.
    sections.forget(4);
    assert!(look(&mut sections, &[(4, [0.5, 70.0, 0.5])]).is_empty());
    // Another world open reuses ids: what the last one saw is dropped.
    sections.begin(Some(NativeWorldEpoch::new(2)));
    sections.see(4, [100.0, 70.0, 0.5]);
    sections.end();
    assert!(sections.crossed.is_empty());
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn no_listener_looks_at_no_entity() {
    with_game(|game| {
        game.spawn_mob_spawned(
            "cow",
            [1.0, 70.0, 3.0],
            crate::game::creatures::SpawnSite::Plain,
        )
        .unwrap();
        game.native_entity_sections();
        assert!(game.entity_sections.seen.is_empty());
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_server_mob_is_told_for_walking_and_teleporting_but_not_for_staying() {
    observe(|calls| {
        with_game(|game| {
            let id = game
                .spawn_mob_spawned(
                    "cow",
                    [1.0, 70.0, 3.0],
                    crate::game::creatures::SpawnSite::Plain,
                )
                .unwrap();
            let walk = |game: &mut crate::game::Game<'_>, at: [f64; 3]| {
                let i = game.mobs.iter().position(|m| m.id == id).unwrap();
                game.mobs
                    .update(i, |m| m.position = Vec3::new(at[0], at[1], at[2]));
            };
            game.native_entity_sections();
            walk(game, [2.0, 70.5, 4.0]);
            game.native_entity_sections();
            assert!(calls.lock().unwrap().is_empty());
            walk(game, [-0.25, 70.5, 4.0]);
            game.native_entity_sections();
            game.teleport_to(Target::Mob(id), [-0.25, 130.0, 4.0], None, true);
            game.native_entity_sections();
            game.native_entity_sections();
            assert_eq!(
                calls.lock().unwrap().as_slice(),
                &[
                    (false, id, [0, 4, 0], [-1, 4, 0], true),
                    (false, id, [-1, 4, 0], [-1, 8, 0], false),
                ]
            );
        })
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_client_is_told_when_its_server_moves_an_entity_into_another_section() {
    use horizon_protocol::packets::play::{self, Clientbound as C};
    observe(|calls| {
        with_client_game(|game| {
            let (pipe, _server) = horizon_protocol::connection::Pipe::pair();
            game.connect_local(pipe, "Steve").unwrap();
            game.handle_packet(C::Login(play::Login {
                player_id: 7,
                hardcore: false,
                levels: vec!["minecraft:overworld".into()],
                max_players: 20,
                chunk_radius: 8,
                simulation_distance: 8,
                reduced_debug_info: false,
                show_death_screen: true,
                limited_crafting: false,
                spawn: play::SpawnInfo {
                    dimension_type: 0,
                    dimension: "minecraft:overworld".into(),
                    seed: 1,
                    game_type: 0,
                    previous_game_type: None,
                    debug: false,
                    flat: false,
                    last_death: None,
                    portal_cooldown: 0,
                    sea_level: 63,
                },
                online_mode: false,
                enforces_secure_chat: false,
            }))
            .unwrap();
            let pig = game
                .remote
                .as_ref()
                .unwrap()
                .builtin
                .get("minecraft:entity_type")
                .unwrap()
                .id("minecraft:pig")
                .unwrap();
            game.handle_packet(C::AddEntity(play::AddEntity {
                id: 20,
                uuid: 20,
                kind: pig,
                position: [1.0, 70.0, 3.0],
                velocity: [0.0; 3],
                pitch: 0,
                yaw: 0,
                head_yaw: 0,
                data: 0,
            }))
            .unwrap();
            let sync = |at: [f64; 3]| {
                C::EntityPositionSync(play::EntityPositionSync {
                    id: 20,
                    path: play::PositionPath::Linear(at),
                    yaw: 0.0,
                    pitch: 0.0,
                    on_ground: true,
                })
            };
            game.native_entity_sections();
            game.handle_packet(sync([3.0, 70.0, 3.0])).unwrap();
            game.native_entity_sections();
            assert!(calls.lock().unwrap().iter().all(|c| c.1 != 20));
            game.handle_packet(sync([3.0, 70.0, 17.0])).unwrap();
            game.native_entity_sections();
            game.native_entity_sections();
            let told: Vec<_> = calls
                .lock()
                .unwrap()
                .iter()
                .filter(|c| c.1 == 20)
                .copied()
                .collect();
            assert_eq!(told, [(true, 20, [0, 4, 0], [0, 4, 1], true)]);
        })
    });
}
