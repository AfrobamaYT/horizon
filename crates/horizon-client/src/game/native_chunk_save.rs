//! Full-chunk save seam. Event dispatch will be added separately.
use super::Game;

#[cfg(test)]
thread_local! {
    static OBSERVER: std::cell::RefCell<Option<Box<dyn for<'a> FnMut(&mut Game<'a>, [i32; 2])>>> =
        std::cell::RefCell::new(None);
}

impl Game<'_> {
    /// Dispatch while the selected dirty chunks are still live, before snapshotting.
    pub(super) fn native_full_chunk_saves(&mut self, keys: &[(i32, i32)]) {
        if self.remote.is_some() {
            return;
        }
        for &(x, z) in keys {
            if self.world.modified.contains(&(x, z)) && self.world.chunks.contains_key(&(x, z)) {
                self.native_full_chunk_save([x, z]);
            }
        }
    }

    fn native_full_chunk_save(&mut self, pos: [i32; 2]) {
        #[cfg(test)]
        OBSERVER.with(|observer| {
            if let Some(observer) = observer.borrow_mut().as_mut() {
                observer(self, pos);
            }
        });
        #[cfg(not(test))]
        let _ = pos;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_client::{
        chunk::{Chunk, LevelShape},
        save::SaveDir,
    };
    use std::{cell::RefCell, rc::Rc, sync::Arc};

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn saves_and_unloads_snapshot_dispatch_changes_to_live_chunks() {
        super::super::native_entity_refs::tests::with_game(|game| {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target")
                .join(format!("chunk-save-seam-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            game.world_root = dir.clone();
            game.save = Arc::new(SaveDir::new(dir.clone(), LevelShape::OVERWORLD));
            let key = (99, 99);
            game.world
                .chunks
                .insert(key, Arc::new(Chunk::empty(key.0, key.1)));
            game.world
                .chunks
                .insert((98, 99), Arc::new(Chunk::empty(98, 99)));
            game.world.modified.insert(key);
            let stone = game
                .pack
                .assets
                .registry
                .default_state("minecraft:stone")
                .unwrap()
                .0;
            let calls = Rc::new(RefCell::new(Vec::new()));
            let seen = calls.clone();
            OBSERVER.with(|observer| {
                *observer.borrow_mut() = Some(Box::new(move |game, pos| {
                    assert_eq!(pos, [99, 99]);
                    let chunk = game
                        .world
                        .chunks
                        .get_mut(&(pos[0], pos[1]))
                        .expect("chunk is live");
                    Arc::make_mut(chunk).set(0, 0, 0, stone);
                    seen.borrow_mut().push(pos);
                }))
            });
            game.save_level(true).unwrap();
            let saved = game
                .save
                .load(key.0, key.1, game.pack.assets.registry, world_biomes!(game))
                .unwrap();
            assert_eq!(saved.get(0, 0, 0), stone);
            assert_eq!(calls.borrow().len(), 1);
            Arc::make_mut(game.world.chunks.get_mut(&key).unwrap()).set(0, 0, 0, 0);
            game.world.modified.insert(key);
            let (reply, _receive) = std::sync::mpsc::channel();
            game.begin_control_save(false, reply).unwrap();
            for _ in 0..5000 {
                game.poll_closing_save().unwrap();
                if game.pending_save.is_none() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            assert!(game.pending_save.is_none());
            let saved = game
                .save
                .load(key.0, key.1, game.pack.assets.registry, world_biomes!(game))
                .unwrap();
            assert_eq!(saved.get(0, 0, 0), stone);
            assert_eq!(calls.borrow().len(), 2);
            Arc::make_mut(game.world.chunks.get_mut(&key).unwrap()).set(0, 0, 0, 0);
            game.world.modified.insert(key);
            game.streaming.paused = false;
            game.streaming.radius = -1;
            game.update_world();
            game.save.flush().unwrap();
            assert!(!game.world.chunks.contains_key(&key));
            assert!(!game.world.chunks.contains_key(&(98, 99)));
            let saved = game
                .save
                .load(key.0, key.1, game.pack.assets.registry, world_biomes!(game))
                .unwrap();
            assert_eq!(saved.get(0, 0, 0), stone);
            assert_eq!(calls.borrow().len(), 3);
            OBSERVER.with(|observer| *observer.borrow_mut() = None);
            let _ = std::fs::remove_dir_all(dir);
        });
    }
}
