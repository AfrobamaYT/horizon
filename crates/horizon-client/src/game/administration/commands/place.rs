//! `/place template` (PlaceCommand): a saved structure template placed with
//! rotation, mirroring and integrity.
use super::{Arg, Context, Game, R, err};
use crate::game::structure_blocks::{load_settings, template_size};
use horizon_client::text::Text;
use horizon_core::position::BlockPos;
use horizon_world::{
    compression::Compression,
    nbt::{self, Limits, Tag},
    worldgen::structure::{Mirror, Rotation, StateTransforms},
};
use std::sync::OnceLock;

fn num(n: impl std::fmt::Display) -> Text {
    Text::literal(n.to_string())
}

fn read_template_data(
    saved: &std::path::Path,
    prepared: &std::path::Path,
    resource: Option<horizon_assets::resource_source::Resource>,
) -> Option<Tag> {
    let saved = std::iter::once_with(|| std::fs::read(saved).ok());
    let resources = std::iter::once_with(|| match resource {
        Some(resource) => resource.read().ok(),
        None => std::fs::read(prepared).ok(),
    });
    for bytes in saved.chain(resources).flatten() {
        let Ok(raw) = Compression::Gzip.decode(&bytes, 1 << 28) else {
            continue;
        };
        if let Ok((named, _)) = nbt::read_named(&raw, Limits::templates()) {
            return Some(named.tag);
        }
    }
    None
}

impl Game<'_> {
    pub(in crate::game) fn state_transforms(&self) -> Option<&'static StateTransforms> {
        static TRANSFORMS: OnceLock<Option<StateTransforms>> = OnceLock::new();
        TRANSFORMS
            .get_or_init(|| {
                let json = std::fs::read_to_string(
                    self.options
                        .resources
                        .join("worldgen/blocks/structure-blocks.json"),
                )
                .ok()?;
                StateTransforms::from_json(&json, self.pack.assets.registry.len()).ok()
            })
            .as_ref()
    }
    /// A saved world template, or the winning passive/world-pack resource.
    pub(in crate::game) fn read_template_tag(&self, id: &str) -> Option<Tag> {
        use horizon_assets::resource_source::{Resource, stays_inside};
        let (ns, path) = id.split_once(':').unwrap_or(("minecraft", id));
        if !stays_inside(ns) || !stays_inside(path) {
            return None;
        }
        let saved = self
            .world_root
            .join("generated")
            .join(ns)
            .join("structures")
            .join(format!("{path}.nbt"));
        let prepared = self
            .options
            .resources
            .join("worldgen/structure")
            .join(ns)
            .join(format!("{path}.nbt"));
        let name = format!("data/{ns}/structure/{path}.nbt");
        let resource = self.fns.data_resource(&name).or_else(|| {
            self.pack
                .data_source
                .contains(&name)
                .then(|| Resource::file(self.pack.data_source.clone(), name))
        });
        read_template_data(&saved, &prepared, resource)
    }
    /// `BlockPosArgument.getLoadedBlockPos(pos)`, or the source's block.
    fn place_pos(&self, ctx: &Context, name: &str) -> R<[i32; 3]> {
        if ctx.get(name).is_some() {
            return self.block_pos_arg(ctx, name);
        }
        Ok(self.source_position().map(|v| v.floor() as i32))
    }
    /// `PlaceCommand.checkLoaded`: every chunk of the inclusive range.
    fn place_check_loaded(&self, min: [i32; 2], max: [i32; 2]) -> R<()> {
        let dimension = self.command_source.map_or(self.dim, |s| s.dimension);
        for cx in min[0]..=max[0] {
            for cz in min[1]..=max[1] {
                if dimension != self.dim || !self.world.chunks.contains_key(&(cx, cz)) {
                    return Err(err("argument.pos.unloaded", &[]));
                }
            }
        }
        Ok(())
    }
    /// `PlaceCommand.placeFeature`, `placeJigsaw` and `placeStructure`.
    pub(super) fn cmd_place_generated(&mut self, id: &str, ctx: &Context) -> R<i32> {
        match id {
            "place.feature" => {
                let raw = match ctx.get("feature") {
                    Some(Arg::Resource(s) | Arg::Str(s)) => s.clone(),
                    _ => return Ok(0),
                };
                let pos = self.place_pos(ctx, "pos")?;
                let (cx, cz) = (pos[0] >> 4, pos[2] >> 4);
                self.place_check_loaded([cx - 1, cz - 1], [cx + 1, cz + 1])?;
                // Inline features are not built by the live feature registry.
                if raw.starts_with('{') {
                    return Err(err("command.failed", &[]));
                }
                let name = if raw.contains(':') {
                    raw
                } else {
                    format!("minecraft:{raw}")
                };
                let at = horizon_core::position::BlockPos::new(pos[0], pos[1], pos[2]);
                let placed = self
                    .with_level(|b, level, ticks| b.place_feature(level, ticks, &name, at))
                    .ok_or_else(|| err("command.failed", &[]))?;
                self.process_level_events()
                    .map_err(|_| err("command.failed", &[]))?;
                if !placed {
                    return Err(err("commands.place.feature.failed", &[]));
                }
                self.feedback(Text::translate(
                    "commands.place.feature.success",
                    &[Text::literal(name), num(pos[0]), num(pos[1]), num(pos[2])],
                ));
                Ok(1)
            }
            "place.jigsaw" => {
                let Some(Arg::Resource(pool)) = ctx.get("pool").cloned() else {
                    return Ok(0);
                };
                if !self
                    .ui
                    .registries
                    .ids("minecraft:worldgen/template_pool")
                    .contains(&pool)
                {
                    return Err(err("commands.place.jigsaw.invalid", &[&pool]));
                }
                let target = match ctx.get("target") {
                    Some(Arg::Resource(s) | Arg::Str(s)) => s.clone(),
                    _ => return Ok(0),
                };
                let max_depth = match ctx.get("max_depth") {
                    Some(Arg::Int(n)) => *n,
                    _ => return Ok(0),
                };
                let pos = self.place_pos(ctx, "position")?;
                self.place_check_loaded([pos[0] >> 4, pos[2] >> 4], [pos[0] >> 4, pos[2] >> 4])?;
                let terrain = self
                    .end
                    .locator()
                    .ok_or_else(|| err("command.failed", &[]))?;
                let query = horizon_world::worldgen::generator::JigsawQuery {
                    pool: &pool,
                    target: &target,
                    max_depth,
                    position: pos,
                };
                let (placed, entities) = self.with_level(|b, level, ticks| {
                    b.place_jigsaw(level, ticks, pos, false, || terrain.jigsaw_pieces(&query))
                });
                for entity in entities {
                    self.spawn_entity_from_nbt(Tag::Compound(entity));
                }
                self.process_level_events()
                    .map_err(|_| err("command.failed", &[]))?;
                if !placed {
                    return Err(err("commands.place.jigsaw.failed", &[]));
                }
                self.feedback(Text::translate(
                    "commands.place.jigsaw.success",
                    &[num(pos[0]), num(pos[1]), num(pos[2])],
                ));
                Ok(1)
            }
            _ => {
                let Some(Arg::Resource(structure)) = ctx.get("structure").cloned() else {
                    return Ok(0);
                };
                if !self
                    .ui
                    .registries
                    .ids("minecraft:worldgen/structure")
                    .contains(&structure)
                {
                    return Err(err("commands.place.structure.invalid", &[&structure]));
                }
                let pos = self.place_pos(ctx, "pos")?;
                // PlaceCommand.placeStructure: Structure.generate for the
                // position's chunk with its `biome -> true` predicate, then
                // placeInChunk for every chunk of the start's box.
                let terrain = self
                    .end
                    .locator()
                    .ok_or_else(|| err("command.failed", &[]))?;
                let start = terrain
                    .structure_start(&structure, pos[0] >> 4, pos[2] >> 4)
                    .ok_or_else(|| err("commands.place.structure.failed", &[]))?;
                let (min, max) = (start.bounding_box.min, start.bounding_box.max);
                self.place_check_loaded([min[0] >> 4, min[2] >> 4], [max[0] >> 4, max[2] >> 4])?;
                let (placed, entities) =
                    self.with_level(|b, level, ticks| b.place_structure(level, ticks, &start));
                for entity in entities {
                    self.spawn_entity_from_nbt(Tag::Compound(entity));
                }
                if !placed {
                    return Err(err("commands.place.structure.failed", &[]));
                }
                self.feedback(Text::translate(
                    "commands.place.structure.success",
                    &[
                        Text::literal(structure),
                        num(pos[0]),
                        num(pos[1]),
                        num(pos[2]),
                    ],
                ));
                Ok(1)
            }
        }
    }
    pub(super) fn cmd_place(&mut self, ctx: &Context) -> R<i32> {
        let Some(Arg::Resource(id)) = ctx.get("template").cloned() else {
            return Ok(0);
        };
        let pos = self.place_pos(ctx, "pos")?;
        let rotation = match ctx.get("rotation") {
            Some(Arg::Str(r)) => Rotation::from_name(r).unwrap_or(Rotation::None),
            _ => Rotation::None,
        };
        let mirror = match ctx.get("mirror") {
            Some(Arg::Str(m)) => match m.as_str() {
                "left_right" => Mirror::LeftRight,
                "front_back" => Mirror::FrontBack,
                _ => Mirror::None,
            },
            _ => Mirror::None,
        };
        let integrity = match ctx.get("integrity") {
            Some(Arg::Float(f)) => *f,
            _ => 1.0,
        };
        let seed = match ctx.get("seed") {
            Some(Arg::Int(s)) => *s,
            _ => 0,
        };
        let strict = ctx.has("strict");
        let entry = self
            .template_entry(&id)
            .ok_or_else(|| err("commands.place.template.invalid", &[&id]))?;
        let size = template_size(&entry.tag);
        let end = [pos[0] + size[0], pos[2] + size[2]];
        self.place_check_loaded([pos[0] >> 4, pos[2] >> 4], [end[0] >> 4, end[1] >> 4])?;
        // `placeInWorld(level, pos, pos, settings, createRandom(seed), flags)`.
        let settings = load_settings(mirror, rotation, integrity, i64::from(seed), strict);
        let at = BlockPos::new(pos[0], pos[1], pos[2]);
        if !self.place_template_entry(&entry, at, settings, mirror, rotation) {
            return Err(err("commands.place.template.failed", &[]));
        }
        self.feedback(Text::translate(
            "commands.place.template.success",
            &[Text::literal(id), num(pos[0]), num(pos[1]), num(pos[2])],
        ));
        Ok(1)
    }
}

#[cfg(test)]
mod source_tests {
    use super::*;
    use horizon_assets::resource_source::{Resource, ResourceSource};
    use std::{io::Write, path::PathBuf};

    #[test]
    fn template_tags_read_archives_below_saved_world_structures() {
        let root = std::env::temp_dir().join(format!(
            "horizon-command-template-source-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        struct Scratch(PathBuf);
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let scratch = Scratch(root);
        let bytes = |value: &str| {
            let named = nbt::NamedTag {
                name: "".into(),
                tag: Tag::compound([("marker", Tag::String(value.into()))]),
            };
            Compression::Gzip
                .encode(&nbt::write_named(&named, Limits::templates()).unwrap())
                .unwrap()
        };
        let prepared = scratch.0.join("prepared.nbt");
        let saved = scratch.0.join("saved/probe.nbt");
        let jar = scratch.0.join("templates.jar");
        std::fs::write(&prepared, bytes("base")).unwrap();
        {
            let mut archive = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
            archive
                .start_file(
                    "data/example/structure/probe.nbt",
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            archive.write_all(&bytes("archive")).unwrap();
            archive.finish().unwrap();
        }
        let source = ResourceSource::archive(&jar).unwrap();
        let resource = Resource::file(source, "data/example/structure/probe.nbt".into());
        let marker = |resource| {
            read_template_data(&saved, &prepared, resource)
                .unwrap()
                .get("marker")
                .cloned()
                .unwrap()
        };
        assert_eq!(
            marker(Some(resource.clone())),
            Tag::String("archive".into())
        );
        std::fs::create_dir_all(saved.parent().unwrap()).unwrap();
        std::fs::write(&saved, bytes("saved")).unwrap();
        assert_eq!(marker(Some(resource.clone())), Tag::String("saved".into()));
        std::fs::write(&saved, b"invalid gzip").unwrap();
        assert_eq!(
            marker(Some(resource.clone())),
            Tag::String("archive".into())
        );
        std::fs::remove_file(&saved).unwrap();
        assert!(
            read_template_data(
                &saved,
                &prepared,
                Some(Resource::bytes(b"invalid gzip".to_vec()))
            )
            .is_none()
        );
        assert_eq!(marker(None), Tag::String("base".into()));
        std::fs::remove_file(&jar).unwrap();
        assert_eq!(marker(Some(resource)), Tag::String("archive".into()));
        assert!(!scratch.0.join("data").exists());
    }
}
