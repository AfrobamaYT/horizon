//! `player_move`: a changed report is held for the mods, ruled on, and the inbox waits behind it.
use super::*;
use crate::lan::guest::InboxItem;

fn with_input_context<T>(run: impl FnOnce(&mut TickCtx<'_>) -> T) -> T {
    let mut world = World::new(horizon_world::registry::BlockStateId(0), 0);
    let mut next_entity = 2;
    let border = crate::worldborder::WorldBorder::default();
    run(&mut TickCtx {
        sea_level: 63,
        world: &mut world,
        entities: &[],
        next_entity: &mut next_entity,
        spawn: [0.5, 64.0, 0.5],
        spawn_angle: [0.0; 2],
        difficulty: 2,
        difficulty_locked: false,
        clock_rate: 1.0,
        tick_rate: 20.0,
        tick_frozen: false,
        tick_steps: Default::default(),
        collision: None,
        border: &border,
        streaming: None,
    })
}

fn at(x: f64) -> S {
    S::MovePlayerPos(play::MovePlayerPos {
        x,
        y: 64.0,
        z: 0.0,
        flags: 1,
    })
}

fn turned(yaw: f32) -> S {
    S::MovePlayerRot(play::MovePlayerRot {
        yaw,
        pitch: 0.0,
        flags: 1,
    })
}

/// A hub whose guest 0 stands at `(0, 64, 0)`, has moved before and listens (or not).
fn standing(listened: bool, inbox: Vec<S>) -> Hub {
    let (mut hub, _rx) = hub_with_a_guest();
    hub.moves_listened = listened;
    hub.guests.update(0, |g| {
        g.load_timeout = 0;
        g.position = [0.0, 64.0, 0.0];
        g.first_good = g.position;
        g.yaw = 0.0;
        g.pitch = 0.0;
        g.ticks = 1;
        g.inbox.extend(inbox.into_iter().map(InboxItem::uncredited));
    });
    hub
}

fn drain(hub: &mut Hub) -> Vec<HostEvent> {
    with_input_context(|ctx| hub.drain_packets(ctx))
}

fn moves(events: &[HostEvent]) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            HostEvent::PlayerMove(conn) => Some(*conn),
            _ => None,
        })
        .collect()
}

fn next_client_tick(hub: &mut Hub, more: Vec<S>) {
    hub.guests.update(0, |g| {
        g.inbox.push_front(InboxItem::uncredited(S::ClientTickEnd(
            horizon_protocol::packets::Empty,
        )));
        g.inbox.extend(more.into_iter().map(InboxItem::uncredited));
    });
}

#[test]
fn two_reports_are_heard_in_order_each_after_the_one_before_was_ruled_on() {
    let mut hub = standing(true, vec![at(0.5), at(0.75)]);
    let conn = hub.guests[0].conn;
    let events = drain(&mut hub);
    assert_eq!(
        moves(&events),
        [conn],
        "one report is held, the second waits behind it"
    );
    let g = &hub.guests[0];
    assert_eq!(
        g.position,
        [0.0, 64.0, 0.0],
        "nothing is committed before the ruling"
    );
    assert_eq!(g.inbox.len(), 1, "the later report keeps its place");
    let first = hub.pending_move_report(conn).unwrap();
    assert_eq!((first.from, first.to), ([0.0, 64.0, 0.0], [0.5, 64.0, 0.0]));
    hub.resolve_move(conn, true);
    assert_eq!(
        hub.guests[0].position,
        [0.5, 64.0, 0.0],
        "a commit applies the report"
    );
    assert!(hub.pending_move_report(conn).is_none());
    // The next client tick: its tick end first, then the report that waited.
    next_client_tick(&mut hub, vec![]);
    let events = drain(&mut hub);
    assert_eq!(moves(&events), [conn]);
    let second = hub.pending_move_report(conn).unwrap();
    assert_eq!(
        (second.from, second.to),
        ([0.5, 64.0, 0.0], [0.75, 64.0, 0.0])
    );
}

#[test]
fn a_veto_drops_the_report_and_corrects_the_client() {
    let mut hub = standing(true, vec![at(0.5)]);
    let conn = hub.guests[0].conn;
    drain(&mut hub);
    let counter = hub.guests[0].teleport_counter;
    hub.resolve_move(conn, false);
    let g = &hub.guests[0];
    assert_eq!(g.position, [0.0, 64.0, 0.0], "the report is not applied");
    assert_eq!(
        g.teleport_counter,
        counter + 1,
        "the client got the teleport correction"
    );
    assert_eq!(
        g.awaiting_teleport.map(|(_, at)| at),
        Some([0.0, 64.0, 0.0])
    );
    assert!(g.pending_move.is_none());
    assert!(g.moves.is_empty(), "its bookkeeping was skipped");
}

#[test]
fn a_report_that_changes_nothing_is_not_held() {
    let mut hub = standing(true, vec![at(0.0)]);
    let events = drain(&mut hub);
    assert!(moves(&events).is_empty(), "the same position is not heard");
    assert!(hub.guests[0].pending_move.is_none());
}

#[test]
fn a_turn_alone_is_held() {
    let mut hub = standing(true, vec![turned(30.0)]);
    let conn = hub.guests[0].conn;
    assert_eq!(moves(&drain(&mut hub)), [conn]);
    let report = hub.pending_move_report(conn).unwrap();
    assert_eq!(report.from, report.to, "only the look changed");
    assert_eq!(
        (report.from_rotation, report.to_rotation),
        ([0.0, 0.0], [30.0, 0.0])
    );
}

#[test]
fn with_no_listener_the_report_is_committed_in_place_and_nothing_is_held() {
    let mut hub = standing(false, vec![at(0.5)]);
    let events = drain(&mut hub);
    assert!(moves(&events).is_empty());
    let g = &hub.guests[0];
    assert_eq!(g.position, [0.5, 64.0, 0.0]);
    assert!(g.pending_move.is_none());
    assert!(g.inbox.is_empty());
}

#[test]
fn a_players_first_move_is_not_heard() {
    let mut hub = standing(true, vec![at(0.5)]);
    hub.guests.update(0, |g| g.ticks = 0);
    assert!(moves(&drain(&mut hub)).is_empty());
    assert_eq!(hub.guests[0].position, [0.5, 64.0, 0.0]);
}

#[test]
fn a_report_nobody_rules_on_is_dropped_with_the_correction_never_committed() {
    let mut hub = standing(true, vec![at(0.5)]);
    let conn = hub.guests[0].conn;
    drain(&mut hub);
    hub.ticks += 21;
    next_client_tick(&mut hub, vec![]);
    drain(&mut hub);
    let g = &hub.guests[0];
    assert!(g.pending_move.is_none());
    assert_eq!(g.position, [0.0, 64.0, 0.0], "never committed");
    assert!(g.awaiting_teleport.is_some(), "the client was corrected");
    hub.resolve_move(conn, true);
    assert_eq!(
        hub.guests[0].position,
        [0.0, 64.0, 0.0],
        "a late ruling finds nothing"
    );
}

#[test]
fn a_teleport_or_a_level_change_drops_a_pending_report_without_a_correction() {
    let mut hub = standing(true, vec![at(0.5)]);
    let conn = hub.guests[0].conn;
    drain(&mut hub);
    hub.teleport_command(0, [10.5, 70.0, 10.5], [false; 3]);
    assert!(hub.guests[0].pending_move.is_none());
    let counter = hub.guests[0].teleport_counter;
    hub.resolve_move(conn, true);
    assert_eq!(
        hub.guests[0].position,
        [10.5, 70.0, 10.5],
        "the teleport stands"
    );
    assert_eq!(
        hub.guests[0].teleport_counter, counter,
        "no second correction"
    );

    let mut hub = standing(true, vec![at(0.5)]);
    drain(&mut hub);
    hub.guests
        .update(0, |g| g.dimension = "minecraft:the_nether".into());
    let counter = hub.guests[0].teleport_counter;
    hub.resolve_move(conn, true);
    let g = &hub.guests[0];
    assert_eq!(
        g.position,
        [0.0, 64.0, 0.0],
        "not committed into another level"
    );
    assert_eq!(g.teleport_counter, counter, "and not corrected");
}

#[test]
fn a_player_who_left_has_nothing_pending_and_a_late_ruling_is_ignored() {
    let mut hub = standing(true, vec![at(0.5)]);
    let conn = hub.guests[0].conn;
    drain(&mut hub);
    let gone = hub.remove_guest(0, "gone");
    assert!(gone.pending_move.is_some(), "it leaves with the guest");
    hub.resolve_move(conn, true);
    hub.resolve_move(conn, false);
    assert!(hub.pending_move_report(conn).is_none());
}
