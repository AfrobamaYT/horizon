//! Passages within one level (#3775, docs/modding/src/how-to/passages.md): the server's
//! registry a mod fills through `LevelRef::register_passage`, its sync to the Horizon
//! clients that hold each passage's chunk (`horizon:passage_sync`, sent with the chunk
//! through the hub's chunk watches), and the copy a client keeps until it forgets the chunk.
use super::Game;
use horizon_client::fast_hash::FxHashMap;
use horizon_protocol::passage_sync::{Message, PassageForm};
use mod_api::{
    DimensionKey, LevelRef, Passage, PassageId, PassageRegistrationError as Refused, PassageTurn,
    Vec3,
};

/// Passages a level holds at most. A client holds every passage of the chunks it has, so
/// this bounds its copy and what the server sends it whatever the view distance.
const MAX_PER_LEVEL: u32 = 256;
/// Passages one chunk holds at most: what the server sends with the chunk, and the
/// passages a client looks through around a moving player (#3775).
const MAX_PER_CHUNK: u32 = 8;
/// The longest side of an opening, in blocks: an opening stays within 23 blocks of its
/// centre, so the chunk holding the centre finds it.
const MAX_SIDE: u32 = 32;
/// How far from 1 a unit axis's squared length, and from 0 the axes' dot product, may be.
const AXIS_TOLERANCE: f64 = 1e-6;
/// `ServerGamePacketListenerImpl.clampHorizontal` and `clampVertical`.
const HORIZONTAL_LIMIT: f64 = 3.0e7;
const VERTICAL_LIMIT: f64 = 2.0e7;

/// The passages of the server's levels and their changes still to tell, and on a client
/// the passages its server told it of.
#[derive(Default)]
pub(crate) struct Passages {
    /// The server's passages, by level, then by chunk ([`Passage::chunk`]).
    levels: FxHashMap<DimensionKey, LevelPassages>,
    /// The last number given; numbers start at 1 and are never reused.
    last: u64,
    /// Registrations and removals not yet told to the clients, in order.
    pending: Vec<Change>,
    /// A client's copy: what its server told it of, kept with the chunk.
    held: FxHashMap<PassageId, Passage>,
}

#[derive(Default)]
struct LevelPassages {
    count: u32,
    chunks: FxHashMap<[i32; 2], Vec<(PassageId, Passage)>>,
}

enum Change {
    Added(DimensionKey, PassageId),
    Removed(DimensionKey, [i32; 2], PassageId),
}

impl Game<'_> {
    /// `LevelRef::register_passage`: checks `passage`, numbers it and keeps it under its
    /// chunk; the clients holding that chunk hear of it in `flush_passages`.
    pub(super) fn level_register_passage(
        &mut self,
        level: LevelRef,
        passage: Passage,
    ) -> Result<PassageId, Refused> {
        self.native_level_world_result(level)?;
        check(&passage)?;
        let chunk = passage.chunk();
        let passages = &mut self.passages;
        let held = passages.levels.entry(level.dimension()).or_default();
        if held.count >= MAX_PER_LEVEL {
            return Err(Refused::LevelFull {
                limit: MAX_PER_LEVEL,
            });
        }
        if held.chunks.get(&chunk).map_or(0, Vec::len) >= MAX_PER_CHUNK as usize {
            return Err(Refused::ChunkFull {
                limit: MAX_PER_CHUNK,
            });
        }
        passages.last += 1;
        let id = PassageId::from_raw(passages.last);
        held.chunks.entry(chunk).or_default().push((id, passage));
        held.count += 1;
        passages.pending.push(Change::Added(level.dimension(), id));
        Ok(id)
    }

    /// `LevelRef::remove_passage`: true when `id` was one of the level's passages; the
    /// clients holding its chunk hear it is gone in `flush_passages`.
    pub(super) fn level_remove_passage(&mut self, level: LevelRef, id: PassageId) -> bool {
        if self.native_level_world_result(level).is_err() {
            return false;
        }
        let Some(held) = self.passages.levels.get_mut(&level.dimension()) else {
            return false;
        };
        let Some((&chunk, list)) = held
            .chunks
            .iter_mut()
            .find(|(_, list)| list.iter().any(|(held, _)| *held == id))
        else {
            return false;
        };
        list.retain(|(held, _)| *held != id);
        if list.is_empty() {
            held.chunks.remove(&chunk);
        }
        held.count -= 1;
        self.passages
            .pending
            .push(Change::Removed(level.dimension(), chunk, id));
        true
    }

    /// The registrations and removals since the last call, told in order to the Horizon
    /// clients that track the passage's chunk (`ChunkMap.isChunkTracked`); from the first one on, the hub
    /// records the chunks it sends (`ChunkWatchLog::record_for_passages`) so that a client
    /// that gets the chunk later hears of it with the chunk. Kept for the next call while
    /// the hub is lent out.
    pub(in crate::game) fn flush_passages(&mut self) {
        if self.passages.pending.is_empty() {
            return;
        }
        let Some(host) = self.lan.as_mut() else {
            return;
        };
        let hub = &mut host.hub;
        hub.chunk_watches.record_for_passages();
        for change in std::mem::take(&mut self.passages.pending) {
            let (key, chunk, message) = match change {
                Change::Added(key, id) => {
                    let Some((chunk, passage)) = self.passages.levels.get(&key).and_then(|l| {
                        l.chunks.iter().find_map(|(chunk, list)| {
                            list.iter()
                                .find(|(held, _)| *held == id)
                                .map(|(_, p)| (*chunk, *p))
                        })
                    }) else {
                        // Removed again before it was told: its removal follows.
                        continue;
                    };
                    (key, chunk, Message::Add(vec![form(id, &passage)]))
                }
                Change::Removed(key, chunk, id) => (key, chunk, Message::Remove(vec![id.raw()])),
            };
            let Some(dimension) = mod_api::__private::dimension_id(key) else {
                continue;
            };
            for g in hub.guests.iter() {
                if g.tracks_chunk(dimension, (chunk[0], chunk[1])) {
                    g.send_passage_sync(&message);
                }
            }
        }
    }

    /// `chunk` of `level` went to player `uuid` (the hub's `chunk_sent`): its passages go
    /// after it, if it has any and the player still tracks it there.
    pub(super) fn send_chunk_passages(&mut self, level: LevelRef, uuid: u128, chunk: [i32; 2]) {
        let Some(list) = self
            .passages
            .levels
            .get(&level.dimension())
            .and_then(|l| l.chunks.get(&chunk))
        else {
            return;
        };
        let Some(host) = self.lan.as_ref() else {
            return;
        };
        let Some(g) = host.hub.guests.iter().find(|g| g.uuid == uuid) else {
            return;
        };
        if g.tracks_chunk(level.dimension_id(), (chunk[0], chunk[1])) {
            let forms = list.iter().map(|(id, p)| form(*id, p)).collect();
            g.send_passage_sync(&Message::Add(forms));
        }
    }

    /// A `horizon:passage_sync` payload the server sent the window.
    pub(super) fn remote_passage_sync(&mut self, data: &[u8]) {
        let message = match horizon_protocol::passage_sync::decode(data) {
            Ok(message) => message,
            Err(e) => return log::debug!("Horizon: a bad passage sync: {e}"),
        };
        let held = &mut self.passages.held;
        match message {
            Message::Add(forms) => {
                for f in forms {
                    held.insert(PassageId::from_raw(f.id), passage(&f));
                }
            }
            Message::Remove(ids) => {
                for id in ids {
                    held.remove(&PassageId::from_raw(id));
                }
            }
        }
    }

    /// The window forgot chunk `(x, z)`: the passages of that chunk go with it.
    pub(super) fn forget_chunk_passages(&mut self, x: i32, z: i32) {
        if !self.passages.held.is_empty() {
            self.passages.held.retain(|_, p| p.chunk() != [x, z]);
        }
    }
}

/// The refusal for a passage that is not a rectangle the engine can carry a player through.
fn check(p: &Passage) -> Result<(), Refused> {
    let parts = |v: Vec3| [v.x, v.y, v.z];
    let finite = [p.origin, p.axis_w, p.axis_h, p.destination]
        .into_iter()
        .flat_map(parts)
        .chain([p.width, p.height])
        .all(f64::is_finite);
    if !finite {
        return Err(Refused::NotFinite);
    }
    let dot = |a: Vec3, b: Vec3| a.x * b.x + a.y * b.y + a.z * b.z;
    let (w, h) = (p.axis_w, p.axis_h);
    if (dot(w, w) - 1.0).abs() > AXIS_TOLERANCE
        || (dot(h, h) - 1.0).abs() > AXIS_TOLERANCE
        || dot(w, h).abs() > AXIS_TOLERANCE
    {
        return Err(Refused::BadAxes);
    }
    let side = |s: f64| s > 0.0 && s <= f64::from(MAX_SIDE);
    if !side(p.width) || !side(p.height) {
        return Err(Refused::BadSize { max: MAX_SIDE });
    }
    let ends = [(p.origin, w, h), (p.destination, p.turned(w), p.turned(h))];
    for (centre, w, h) in ends {
        for (sw, sh) in [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)] {
            let (a, b) = (sw * p.width, sh * p.height);
            let corner = [
                centre.x + w.x * a + h.x * b,
                centre.y + w.y * a + h.y * b,
                centre.z + w.z * a + h.z * b,
            ];
            if corner[0].abs() > HORIZONTAL_LIMIT
                || corner[2].abs() > HORIZONTAL_LIMIT
                || corner[1].abs() > VERTICAL_LIMIT
            {
                return Err(Refused::OutsideWorld);
            }
        }
    }
    Ok(())
}

fn turn_index(turn: PassageTurn) -> u8 {
    match turn {
        PassageTurn::Clockwise90 => 1,
        PassageTurn::Clockwise180 => 2,
        PassageTurn::Counterclockwise90 => 3,
        _ => 0,
    }
}

fn form(id: PassageId, p: &Passage) -> PassageForm {
    let v = |v: Vec3| [v.x, v.y, v.z];
    PassageForm {
        id: id.raw(),
        origin: v(p.origin),
        axis_w: v(p.axis_w),
        axis_h: v(p.axis_h),
        width: p.width,
        height: p.height,
        destination: v(p.destination),
        turn: turn_index(p.turn),
    }
}

fn passage(f: &PassageForm) -> Passage {
    let v = |[x, y, z]: [f64; 3]| Vec3::new(x, y, z);
    Passage {
        origin: v(f.origin),
        axis_w: v(f.axis_w),
        axis_h: v(f.axis_h),
        width: f.width,
        height: f.height,
        destination: v(f.destination),
        turn: match f.turn {
            1 => PassageTurn::Clockwise90,
            2 => PassageTurn::Clockwise180,
            3 => PassageTurn::Counterclockwise90,
            _ => PassageTurn::None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{Message, PassageForm, Refused, check};
    use crate::game::native_entity_refs::tests::{with_client_game, with_game};
    use crate::game::native_guest_turn_tests::with_guests;
    use horizon_client::lan::chunk_watch::{
        ChunkWatch, ChunkWatchKind, ChunkWatchLog, LevelWatches,
    };
    use mod_api::{LevelRef, Passage, PassageId, PassageTurn, Vec3, WorldEpoch};
    use std::sync::{Arc, mpsc};

    /// A 2 by 3 doorway facing south at (0.5, 65.5, 8.5) in chunk (0, 0), coming out 100
    /// blocks east, turned a clockwise quarter.
    fn doorway() -> Passage {
        Passage {
            origin: Vec3::new(0.5, 65.5, 8.5),
            axis_w: Vec3::new(1.0, 0.0, 0.0),
            axis_h: Vec3::new(0.0, 1.0, 0.0),
            width: 2.0,
            height: 3.0,
            destination: Vec3::new(100.5, 65.5, 8.5),
            turn: PassageTurn::Clockwise90,
        }
    }

    /// The doorway as it travels, numbered `id`.
    fn doorway_form(id: u64) -> PassageForm {
        PassageForm {
            id,
            origin: [0.5, 65.5, 8.5],
            axis_w: [1.0, 0.0, 0.0],
            axis_h: [0.0, 1.0, 0.0],
            width: 2.0,
            height: 3.0,
            destination: [100.5, 65.5, 8.5],
            turn: 1,
        }
    }

    /// The passage messages a guest was sent, in order.
    fn passage_messages(rx: &mpsc::Receiver<horizon_client::lan::Out>) -> Vec<Message> {
        use horizon_client::lan::Out;
        use horizon_protocol::{buf::Reader, packets::play::Clientbound};
        let mut messages = Vec::new();
        for out in rx.try_iter() {
            let bodies = match out {
                Out::Packet(p) => vec![p],
                Out::Packets(p) => p,
                Out::Disconnect(_) => panic!("unexpected disconnect"),
            };
            for body in bodies {
                let mut r = Reader::new(&body);
                let id = r.varint().unwrap();
                if let Clientbound::CustomPayload(p) = Clientbound::decode(id, r.rest()).unwrap()
                    && p.channel == "horizon:passage_sync"
                {
                    messages.push(horizon_protocol::passage_sync::decode(&p.data).unwrap());
                }
            }
        }
        messages
    }

    /// The doorway passes; each way of not being a finite rectangle with unit axes at a
    /// right angle, sides in (0, 32] and corners inside vanilla's clamps is refused by name,
    /// at either end.
    #[test]
    fn a_passage_that_is_not_a_finite_rectangle_inside_the_world_is_refused() {
        assert_eq!(check(&doorway()), Ok(()));
        let with = |change: fn(&mut Passage)| {
            let mut p = doorway();
            change(&mut p);
            check(&p)
        };
        assert_eq!(with(|p| p.origin.y = f64::NAN), Err(Refused::NotFinite));
        assert_eq!(with(|p| p.height = f64::INFINITY), Err(Refused::NotFinite));
        assert_eq!(
            with(|p| p.axis_w = Vec3::new(2.0, 0.0, 0.0)),
            Err(Refused::BadAxes)
        );
        assert_eq!(
            with(|p| p.axis_h = Vec3::new(0.6, 0.8, 0.0)),
            Err(Refused::BadAxes)
        );
        assert_eq!(with(|p| p.width = 0.0), Err(Refused::BadSize { max: 32 }));
        assert_eq!(with(|p| p.height = -1.0), Err(Refused::BadSize { max: 32 }));
        assert_eq!(with(|p| p.width = 32.5), Err(Refused::BadSize { max: 32 }));
        assert_eq!(with(|p| p.width = 32.0), Ok(()));
        // The far side's corner: 29,999,999.5 + 1 east is past 30,000,000.
        assert_eq!(
            with(|p| p.destination.x = 29_999_999.5),
            Ok(()),
            "turned a quarter, the width runs north-south there"
        );
        assert_eq!(
            with(|p| {
                p.destination.x = 29_999_999.5;
                p.turn = PassageTurn::None;
            }),
            Err(Refused::OutsideWorld)
        );
        assert_eq!(
            with(|p| p.origin.z = -30_000_000.5),
            Err(Refused::OutsideWorld)
        );
        assert_eq!(
            with(|p| p.origin.y = 20_000_000.0),
            Err(Refused::OutsideWorld)
        );
    }

    /// Passages are numbered from 1 in order; a chunk takes 8 and a level 256, and the next
    /// is refused with nothing registered; a removal frees its place once. A stale handle
    /// and a client register nothing.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_level_numbers_its_passages_and_refuses_past_its_caps() {
        with_game(|game| {
            let level = game.level(game.dim.id()).unwrap();
            let in_chunk = |cx: i32, i: u32| Passage {
                origin: Vec3::new(f64::from(cx * 16) + 0.5, 65.5, f64::from(i) + 0.5),
                ..doorway()
            };
            for i in 0..8 {
                let id = game.level_register_passage(level, in_chunk(0, i)).unwrap();
                assert_eq!(id, PassageId::from_raw(u64::from(i) + 1));
            }
            assert_eq!(
                game.level_register_passage(level, in_chunk(0, 8)),
                Err(Refused::ChunkFull { limit: 8 })
            );
            for cx in 1..32 {
                for i in 0..8 {
                    game.level_register_passage(level, in_chunk(cx, i)).unwrap();
                }
            }
            assert_eq!(
                game.level_register_passage(level, in_chunk(40, 0)),
                Err(Refused::LevelFull { limit: 256 })
            );
            assert!(game.level_remove_passage(level, PassageId::from_raw(3)));
            assert!(!game.level_remove_passage(level, PassageId::from_raw(3)));
            assert!(!game.level_remove_passage(level, PassageId::from_raw(999)));
            assert_eq!(
                game.level_register_passage(level, in_chunk(0, 8)),
                Ok(PassageId::from_raw(257)),
                "the freed place, under a new number"
            );
            assert_eq!(
                game.level_register_passage(level, in_chunk(40, 0)),
                Err(Refused::LevelFull { limit: 256 })
            );
            let stale = LevelRef::new(WorldEpoch::new(0), game.dim.key());
            assert_eq!(
                game.level_register_passage(stale, doorway()),
                Err(Refused::Stale)
            );
            assert!(!game.level_remove_passage(stale, PassageId::from_raw(1)));
        });
        with_client_game(|game| {
            let epoch = game.native_lifecycle.epoch().unwrap();
            let level = LevelRef::new(epoch, game.dim.key());
            assert_eq!(
                game.level_register_passage(level, doorway()),
                Err(Refused::Unavailable)
            );
            assert!(!game.level_remove_passage(level, PassageId::from_raw(1)));
        });
    }

    /// At the end of the tick a registration reaches the Horizon client holding the
    /// passage's chunk, not one without it nor a vanilla client; the hub then records the
    /// chunks it sends, and a client that gets the chunk later is sent the passage with it.
    /// A removal reaches both Horizon clients that hold the chunk.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_passage_goes_to_the_horizon_clients_holding_its_chunk() {
        with_guests(|game| {
            game.native_lifecycle.world_open_ready().unwrap();
            let mut receivers = Vec::new();
            for conn in [12, 13] {
                let (tx, rx) = mpsc::channel();
                game.lan.as_mut().unwrap().hub.update_guest(conn, |g| {
                    g.sender = horizon_client::lan::PlayerSender::new(tx, Arc::default());
                    g.brand = "horizon".into();
                });
                receivers.push(rx);
            }
            let hub = &mut game.lan.as_mut().unwrap().hub;
            hub.guests.update_all(|g| g.center = Some((0, 0)));
            hub.update_guest(12, |g| {
                g.sent_chunks.insert((0, 0));
            });
            assert!(!hub.chunk_watches.recording());
            let level = game.level(game.dim.id()).unwrap();
            let id = game.level_register_passage(level, doorway()).unwrap();
            assert!(
                passage_messages(&receivers[0]).is_empty(),
                "told at the tick's end"
            );
            game.flush_passages();
            assert_eq!(
                passage_messages(&receivers[0]),
                [Message::Add(vec![doorway_form(1)])]
            );
            assert!(passage_messages(&receivers[1]).is_empty());
            let dimension = game.dim.id().to_owned();
            let hub = &mut game.lan.as_mut().unwrap().hub;
            assert!(hub.chunk_watches.recording());

            // Guest 13 gets chunk (0, 0): the hub's record of it brings the passage.
            hub.update_guest(13, |g| {
                g.sent_chunks.insert((0, 0));
            });
            let sent = |uuid: u128| {
                ChunkWatchLog::new(vec![LevelWatches {
                    dimension: dimension.clone(),
                    watches: vec![ChunkWatch {
                        kind: ChunkWatchKind::Sent,
                        uuid,
                        entity_id: uuid as i32,
                        chunk: [0, 0],
                    }],
                }])
            };
            hub.chunk_watches = sent(13);
            game.run_chunk_watches();
            assert_eq!(
                passage_messages(&receivers[1]),
                [Message::Add(vec![doorway_form(1)])]
            );

            // A vanilla client is never sent one.
            let hub = &mut game.lan.as_mut().unwrap().hub;
            hub.update_guest(12, |g| g.brand = "vanilla".into());
            hub.chunk_watches = sent(12);
            game.run_chunk_watches();
            assert!(passage_messages(&receivers[0]).is_empty());
            let hub = &mut game.lan.as_mut().unwrap().hub;
            hub.update_guest(12, |g| g.brand = "horizon".into());

            assert!(game.level_remove_passage(level, id));
            game.flush_passages();
            for rx in &receivers {
                assert_eq!(passage_messages(rx), [Message::Remove(vec![1])]);
            }
        });
    }

    /// The window holds what its server sent, by number, until the server removes it or
    /// the window forgets the passage's chunk; another chunk's going leaves it.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn the_window_keeps_a_passage_until_it_forgets_its_chunk() {
        use horizon_protocol::passage_sync::encode;
        with_client_game(|game| {
            let add = encode(&Message::Add(vec![doorway_form(4), doorway_form(5)])).unwrap();
            game.remote_passage_sync(&add);
            assert_eq!(
                game.passages.held.get(&PassageId::from_raw(4)),
                Some(&doorway())
            );
            game.remote_passage_sync(&encode(&Message::Remove(vec![5])).unwrap());
            assert_eq!(game.passages.held.len(), 1);
            game.client_chunk_unload(1, 0, true);
            assert_eq!(game.passages.held.len(), 1);
            game.client_chunk_unload(0, 0, true);
            assert!(game.passages.held.is_empty());
        });
    }
}
