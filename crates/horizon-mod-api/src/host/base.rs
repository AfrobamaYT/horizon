//! The `base` area: host methods the game implements (`game/host/base.rs`).
#![allow(unused_variables)]
use crate::*;
use std::path::PathBuf;
/// The host's methods both sides share of the `base` area, implemented by the game and reached through [`Host`].
pub trait BaseHost {
    /// Side: both.
    /// The server-owned holders' attachments; `TYPE.get(host, holder)` reads
    /// one. On a client it is the copy of what the server synced: `set` and
    /// `remove` change only the copy.
    fn attachments(&mut self) -> &mut dyn AttachmentStore {
        crate::phase::unprovided(
            "Host::attachments",
            "attachments",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Read a loaded, light-emitting block. No implicit chunk loading.
    fn lit_block_name(&self, pos: [i32; 3]) -> Option<&str> {
        crate::phase::unprovided(
            "Host::lit_block_name",
            "lit_block_name",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The most of the item `item` one slot holds (`Item.getDefaultMaxStackSize`),
    /// or `None` for an unknown item: for a mod's container that fills or
    /// merges stacks itself.
    fn max_stack_size(&self, item: &str) -> Option<u32> {
        crate::phase::unprovided(
            "Host::max_stack_size",
            "max_stack_size",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Property `key` of block state `state` (a machine's `facing`, from the
    /// `state` `native_block_entity_submit` gets), if it has one: no lookup in
    /// the world, so cheap each frame.
    fn block_state_property(&self, state: BlockStateId, key: &str) -> Option<&str> {
        crate::phase::unprovided(
            "Host::block_state_property",
            "block_state_property",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether the player left mod `mod_id` on in Options > Mods. A mod
    /// turned off does nothing (check this first in each hook) until the
    /// launcher's next build leaves it out.
    fn on(&self, mod_id: &str) -> bool {
        crate::phase::unprovided("Host::on", "on", crate::phase::LoadAccess::WorldOnly)
    }
    /// Side: both.
    /// A mod's setting `key` (one file per key in `native_data_dir`).
    fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        crate::phase::unprovided(
            "Host::setting",
            "setting",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Keeps a mod's setting `key`.
    fn set_setting(&self, mod_id: &str, key: &str, value: &str) {
        crate::phase::unprovided(
            "Host::set_setting",
            "set_setting",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether a mod with the `horizon-mod.toml` id `id` is installed: a
    /// native, wasm or content package (Fabric's `FabricLoader.isModLoaded`,
    /// NeoForge's `ModList.isLoaded`). The list is fixed at startup.
    fn is_loaded(&self, id: &str) -> bool {
        crate::phase::unprovided(
            "Host::is_loaded",
            "is_loaded",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The calling mod's own files in the game's `config/` folder, beside
    /// `options.txt` (beside `server.properties` on a dedicated server):
    /// its folder `config/<mod id>/` and the files beside it named
    /// `<mod id>.<ext>` or `<mod id>-<rest>`, as Fabric and NeoForge mods
    /// keep theirs. The service is bound to the mod whose hook or part is
    /// running, so a mod never names its id and can't reach another mod's
    /// files; a world's `serverconfig/` is read only through
    /// `config_loading`.
    fn config_files(&self) -> &'static dyn ConfigFiles {
        crate::phase::unprovided(
            "Host::config_files",
            "config_files",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// The recipes of type `kind` (`minecraft:smelting`, `mymod:crusher`), vanilla's
    /// types and mods' own alike, from `data/<namespace>/recipe/*.json` of vanilla,
    /// the content mods (`.hmod`) and the world's data packs, in file order: each
    /// one typed (`Recipe`: its ingredients, results, group, cooking time and
    /// experience, or its stonecutting, smithing or special crafting rule),
    /// with its file as JSON for a mod's own type. Dynamic rules are not fixed
    /// outputs; use `LevelRef::craft` to assemble a crafting grid. These are the
    /// recipes in force (`RecipeManager.getAllRecipesFor`): data packs merged and
    /// load conditions held, so a pack's override or removal shows. A data pack
    /// changes them on `/reload`, so look them up again when you need them. Empty
    /// for a type nobody uses and outside a world. These are the server's table on
    /// the server thread, a dedicated server or a LAN host; a client, the
    /// single-player window included, sees vanilla's and the content mods' recipes,
    /// not the world's data packs', unless a mod had the server send it the type
    /// (`native_datapack_sync`).
    fn recipes_of_type(&self, kind: &str) -> Arc<[Recipe]> {
        crate::phase::unprovided(
            "Host::recipes_of_type",
            "recipes_of_type",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
    /// Side: both.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// An ingredient as a recipe's JSON names it: an item, a `#tag`, or a list
    /// of them, a bare id as `minecraft:` (as vanilla reads it); `test` says
    /// whether a stack fits. Items the game doesn't know are left out; None
    /// when nothing is left (an unknown or empty tag, for one) and outside a
    /// world.
    fn recipe_ingredient(&self, json: &serde_json::Value) -> Option<Ingredient> {
        crate::phase::unprovided(
            "Host::recipe_ingredient",
            "recipe_ingredient",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
    /// Side: both.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// A stack as a recipe's `result` names it: `id` (a bare one as
    /// `minecraft:`), `count` (1 by default, up to 99) and `components`. None
    /// without an `id`, for an item the game doesn't know or a count outside
    /// that, and outside a world.
    fn recipe_result(&self, json: &serde_json::Value) -> Option<ItemStack> {
        crate::phase::unprovided(
            "Host::recipe_result",
            "recipe_result",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
    /// Side: both.
    /// The `book_id`s of the recipes the player's recipe book holds now
    /// (the server's book on a server); empty outside a world.
    fn known_recipes(&mut self) -> Arc<[i32]> {
        crate::phase::unprovided(
            "Host::known_recipes",
            "known_recipes",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The level in place (`minecraft:overworld`), for a hook that isn't told
    /// which it runs in.
    fn dimension(&self) -> &'static str {
        crate::phase::unprovided(
            "Host::dimension",
            "dimension",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The instance (game) folder, which its players share: `saves/`,
    /// `resourcepacks/`, `shaderpacks/` and `horizon-mods/` are in it, and the
    /// title's screens create worlds and read packs there. A world's own
    /// `datapacks/` is in that world's folder (`saves/<world>/datapacks`).
    /// The folder the game runs in (`--player-dir` moves a player's options
    /// and servers only, not the instance). Nothing is created.
    ///
    /// Not on wasm: a path in the game's file system, which a wasm mod has none of; a wasm mod
    /// reads its own data and assets through reload listeners.
    fn instance_dir(&self) -> PathBuf {
        crate::phase::unprovided(
            "Host::instance_dir",
            "instance_dir",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The server host's methods of the `base` area, implemented by the game and reached through [`ServerHost`].
pub trait BaseServerHost {
    /// Side: server.
    /// Kills mob `id` as `/kill` does: it falls and drops its loot. False for
    /// no such living mob or another dimension.
    fn kill_mob(&mut self, dimension: &str, id: u32) -> bool {
        crate::phase::unprovided(
            "ServerHost::kill_mob",
            "kill_mob",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Spawns an entity of `kind` (`minecraft:item_display`) at `pos` in the
    /// level in place, `dimension`, from `nbt` (its compound, as `/summon
    /// <kind> <pos> <nbt>` takes it; passengers included): its id. None on a
    /// client's copy of the world, for another dimension, a position outside
    /// `/summon`'s bounds, a chunk that isn't loaded, a kind Horizon can't
    /// load, or one a mod kept out (`native_entity_join_level`).
    fn summon(
        &mut self,
        dimension: &str,
        kind: &str,
        pos: [f64; 3],
        nbt: Compound,
    ) -> Option<u32> {
        crate::phase::unprovided(
            "ServerHost::summon",
            "summon",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The server's tick rate (20 unless `/tick rate` changed it) and the
    /// milliseconds its ticks took on average lately, as `/tick query`
    /// reports them. The ticks a second it keeps up is the rate, or
    /// 1000 / milliseconds when that is lower.
    fn tick_stats(&self) -> (f32, f64) {
        crate::phase::unprovided(
            "ServerHost::tick_stats",
            "tick_stats",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The open world's folder (`world/`, beside its `level.dat`): a server
    /// mod keeps its data in a folder of its own there, so the world's
    /// backups carry it. None when no world is open.
    ///
    /// Not on wasm: a path in the game's file system, which a wasm mod has none of; a wasm mod
    /// keeps world data in level attachments.
    fn world_dir(&self) -> Option<PathBuf> {
        crate::phase::unprovided(
            "ServerHost::world_dir",
            "world_dir",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// For a server mod: the runs made at once, straight into the chunks of the authoritative level `dimension`
    /// (the one being ticked, as for `native_set_block`), with what `fill ... strict` does for block entities (a
    /// block replaced by another loses its block entity, and a new block gets the one `native_set_block` would
    /// give it, a mod block's included); no neighbour is told, and the chunks changed go to the players whole.
    /// Runs in chunks not loaded, and the parts of runs outside the column, are left out. The blocks changed, or
    /// why the edit was left out, with nothing changed: another dimension, a client of a server, a palette entry
    /// or filter that doesn't read or carries NBT (refused), a run naming a palette entry or filter past the end,
    /// or more than 65536 filters.
    fn edit_blocks(
        &mut self,
        dimension: &str,
        edit: &NativeBlockEdit,
    ) -> Result<u64, String> {
        crate::phase::unprovided(
            "ServerHost::edit_blocks",
            "edit_blocks",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// For a server mod: each column's height by `heightmap` in the authoritative level `dimension` (the one being
    /// ticked), none for a column not loaded. `Err` says why nothing was read: another dimension or a client of a
    /// server, or more than 65,536 columns (`MAX_COLUMNS`).
    fn surface_heights(
        &self,
        dimension: &str,
        columns: &[(i32, i32)],
        heightmap: NativeHeightmap,
    ) -> Result<Vec<Option<i32>>, String> {
        crate::phase::unprovided(
            "ServerHost::surface_heights",
            "surface_heights",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// For a server mod: `below` blocks under and `above` blocks from each column's height by `heightmap`, as a
    /// palette and indexes, with the biome's grass, foliage and water tints (air outside the column and in columns
    /// not loaded). The tints are the biome's at the column's top, not blended over the biome blend radius as the
    /// game draws them: near a biome border they can differ from what players see. `Err` says why nothing was
    /// read: another dimension or a client of a server, `below + above` of 0 or more than the level's height, or
    /// a read of more than 1,048,576 blocks (`MAX_READ`; it runs on the server tick, so read big areas in pieces).
    fn read_columns(
        &self,
        dimension: &str,
        columns: &[(i32, i32)],
        heightmap: NativeHeightmap,
        range: (u32, u32),
    ) -> Result<NativeColumnBlocks, String> {
        crate::phase::unprovided(
            "ServerHost::read_columns",
            "read_columns",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Writes the experience orb `id`'s `value` (the points a pickup gives), `count`
    /// (orbs merged into it, each worth `value`) and `age` (ticks lived; an orb
    /// despawns at 6000); a `None` is left as it is.
    /// False, and nothing written, for an orb not in the server level `dimension`, a
    /// `value` or `count` of 0, and on a client of a server.
    fn set_experience_orb(
        &mut self,
        dimension: &str,
        id: u32,
        value: Option<u32>,
        count: Option<u32>,
        age: Option<u32>,
    ) -> bool {
        crate::phase::unprovided(
            "ServerHost::set_experience_orb",
            "set_experience_orb",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The experience orbs of the server level `dimension` whose box meets
    /// the box `min`..`max`, in the level's order: what a mod reads before it
    /// cancels an orb in `native_entity_join_level`, to give its points with
    /// `PlayerRef::give_experience_points`. Empty for a level not in place and on a
    /// client of a server.
    fn experience_orbs(
        &self,
        dimension: &str,
        min: [f64; 3],
        max: [f64; 3],
    ) -> Vec<NativeExperienceOrb> {
        crate::phase::unprovided(
            "ServerHost::experience_orbs",
            "experience_orbs",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Throws a falling block of `state` from `pos` (its bottom centre) at
    /// `velocity` (blocks a tick) in the level in place, `dimension`: it
    /// falls, lands and places itself as sand does, or drops as an item
    /// where it can't. The block at `pos` is left as it is: take it away first
    /// with `native_set_block` to make a block fall out of the world. False,
    /// with nothing thrown, on a client's copy of the world, for another
    /// dimension, an air or unknown state, a chunk that isn't loaded, a value
    /// that isn't finite, or with 1024 falling blocks in the level already.
    /// Each axis of `velocity` is cut to 3.9.
    fn spawn_falling_block(
        &mut self,
        dimension: &str,
        state: BlockStateId,
        pos: [f64; 3],
        velocity: [f64; 3],
        falling: NativeFalling,
    ) -> bool {
        crate::phase::unprovided(
            "ServerHost::spawn_falling_block",
            "spawn_falling_block",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sends clients the block at `pos` and its block entity's update tag
    /// (`native_block_entity_update_tag`) again, as `Level.sendBlockUpdated`
    /// does: with the level's other block changes, to the players who see its
    /// chunk, once however often it is called before then. Changes made in a
    /// block entity's tick go out at the start of the next tick, as vanilla's
    /// do (`ServerLevel.tick` broadcasts chunks before it ticks block
    /// entities). Call it when what clients show changed (a machine started),
    /// not for every tick's progress: a menu's data slots carry that. Nothing
    /// happens for an unloaded block.
    fn block_entity_sync(&mut self, pos: [i32; 3]) {
        crate::phase::unprovided(
            "ServerHost::block_entity_sync",
            "block_entity_sync",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets property `key` of the block at `pos` to `value`, as a furnace's
    /// tick sets its `lit` (`Level.setBlock` with `UPDATE_ALL`: neighbours
    /// and clients are told); true when the block changed, false when the
    /// block has no such property or value, or the property already has
    /// `value` (as `Level.setBlock` for an unchanged state). The block
    /// entity stays.
    fn set_block_property(&mut self, pos: [i32; 3], key: &str, value: &str) -> bool {
        crate::phase::unprovided(
            "ServerHost::set_block_property",
            "set_block_property",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `native_set_block_property` with `Level.setBlock`'s `flags`, as a
    /// brewing stand's tick sets its bottles with `flags::UPDATE_CLIENTS`.
    /// Players get the write when `UPDATE_CLIENTS` is set; false when this
    /// window isn't a server, and in each case `native_set_block_property`
    /// returns false.
    fn set_block_property_with(
        &mut self,
        pos: [i32; 3],
        key: &str,
        value: &str,
        flags: i32,
    ) -> bool {
        crate::phase::unprovided(
            "ServerHost::set_block_property_with",
            "set_block_property_with",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sends `data` on `channel` (`mod_id:name`, at most 30 000 bytes, at
    /// most 32 a player a tick) to `player`'s client, at the tick's end, from
    /// any server hook. Ok means accepted: a player who left or isn't on a
    /// Horizon client doesn't get it.
    fn net_send_to_player(
        &mut self,
        player: u128,
        channel: &str,
        data: &[u8],
    ) -> Result<(), String> {
        crate::phase::unprovided(
            "ServerHost::net_send_to_player",
            "net_send_to_player",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Drop one item as DefaultDispenseItemBehavior does. For a dispenser hook:
    /// return this remainder in its answer with `CLICK`; the caller plays the
    /// answer's event and smoke once. A client, another dimension or an unloaded
    /// chunk keeps the input stack unchanged.
    fn dispense_default(
        &mut self,
        dimension: &str,
        pos: [i32; 3],
        facing: Direction,
        stack: ItemStack,
    ) -> Option<ItemStack> {
        crate::phase::unprovided(
            "ServerHost::dispense_default",
            "dispense_default",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Opens a chest menu of `rows` rows (1 to 6) titled `title` for the
    /// player, showing `items` (by slot, padded with empty ones), in place of
    /// the menu they had open: its window id.
    fn open_menu(
        &mut self,
        uuid: u128,
        rows: u8,
        title: Text,
        items: Vec<Option<ItemStack>>,
    ) -> Option<i32> {
        crate::phase::unprovided(
            "ServerHost::open_menu",
            "open_menu",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Closes the player's native menu (a `native_menu_closed` follows).
    fn close_menu(&mut self, uuid: u128) {
        crate::phase::unprovided(
            "ServerHost::close_menu",
            "close_menu",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The client host's methods of the `base` area, implemented by the game and reached through [`ClientHost`].
pub trait BaseClientHost {
    /// Side: client.
    /// A small local vanilla particle; caller should rate-limit emission.
    fn flame_particle(&mut self, at: [f64; 3], soul: bool) {
        crate::phase::unprovided(
            "ClientHost::flame_particle",
            "flame_particle",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Asks `native_biome_colors` again for every biome and, when the colours
    /// differ from those in use, rebuilds every loaded section and the far
    /// terrain with them (`LevelRenderer.allChanged`, as changing Biome Blend
    /// does). Call it when your colours change (a new season, a new day of a
    /// slow fade) and once when the player joins, not every frame: a rebuild
    /// takes the workers a few seconds.
    fn refresh_biome_colors(&mut self) {
        crate::phase::unprovided(
            "ClientHost::refresh_biome_colors",
            "refresh_biome_colors",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Opens a screen of mod `mod_id`'s own over the world: the screen hooks
    /// fire over it as over any other, so the mod draws it in
    /// `native_screen_render_post` and takes its clicks, scroll, keys and
    /// typing there; the mouse is freed, and Escape closes it. Every mod's own
    /// screen has the `NativeScreenView.id` `mod`, so a mod's screen hooks
    /// check `native_mod_screen()` is its own before drawing. It doesn't pause
    /// a single-player game (as the inventory doesn't). True when it is open:
    /// false while another screen is (or another mod's), over the title
    /// screen, a book, a sign editor, a dialog or a prompt, while the world
    /// loads, and anywhere but the window's own client (the server thread, a
    /// dedicated server). A gamepad can close it (B, Start) but not click on
    /// it: its clicks don't reach the mouse hooks.
    fn mod_screen_open(&mut self, mod_id: &str) -> bool {
        crate::phase::unprovided(
            "ClientHost::mod_screen_open",
            "mod_screen_open",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Closes mod `mod_id`'s own screen, if it is open.
    fn mod_screen_close(&mut self, mod_id: &str) {
        crate::phase::unprovided(
            "ClientHost::mod_screen_close",
            "mod_screen_close",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// The mod whose own screen is open, if one is (Escape, or another screen
    /// replacing it, closes it).
    fn mod_screen(&self) -> Option<&str> {
        crate::phase::unprovided(
            "ClientHost::mod_screen",
            "mod_screen",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The clock of the world this window shows, for client mods: its game
    /// time and time of day as the server last sent them
    /// (`ClientboundSetTimePacket`), advanced each tick between packets; none
    /// on the title screen.
    fn clock(&self) -> Option<NativeClock> {
        crate::phase::unprovided(
            "ClientHost::clock",
            "clock",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The first block along the view within `distance` (at most 1024), past the loaded chunks over the distant terrain.
    fn raycast(&self, distance: f32) -> Option<NativeHit> {
        crate::phase::unprovided(
            "ClientHost::raycast",
            "raycast",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The first block along the view within `RAYCAST_MAX`, and its distance.
    fn view_hit(&self) -> Option<(f32, NativeHit)> {
        crate::phase::unprovided(
            "ClientHost::view_hit",
            "view_hit",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// Right-clicks with these items (at most 64 ids) are mod `mod_id`'s; a later call replaces the list.
    fn claim_items(&mut self, mod_id: &str, ids: &[String]) {
        crate::phase::unprovided(
            "ClientHost::claim_items",
            "claim_items",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Mod `mod_id`'s events since its last poll (at most 256 wait).
    fn poll(&mut self, mod_id: &str) -> Vec<NativeModEvent> {
        crate::phase::unprovided(
            "ClientHost::poll",
            "poll",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Plays `sound` for this player only (`SoundPlay`'s event: `minecraft:ui.button.click`,
    /// or a mod's own from its `assets/<id>/sounds.json`): nothing is sent to the server, and
    /// other players don't hear it. It takes `&self`, so a screen, key or mouse hook can play
    /// one: the sound is queued and starts at the next audio update (once a client tick),
    /// `after` ticks counted from then (0 to 1200, a minute; `SoundEngine.playDelayed`). `SoundPlay::ui` plays it at the listener as an interface sound
    /// (`SimpleSoundInstance.forUI`); `SoundPlay::at` at a position in the client's level,
    /// fading with distance (`volume` past 1 carries farther, 16 blocks times the volume), as
    /// thunder heard after the flash or a blast heard when its shock wave arrives. False,
    /// with nothing played, on a server or for a value out of range; an event the resources
    /// don't have plays nothing, as vanilla's.
    fn play_local_sound(&self, sound: &SoundPlay, after: u32) -> bool {
        crate::phase::unprovided(
            "ClientHost::play_local_sound",
            "play_local_sound",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The particles alive in the client's level (`ParticleEngine.countParticles`): 0 once
    /// a clear is asked, and on a server.
    fn particle_count(&self) -> u32 {
        crate::phase::unprovided(
            "ClientHost::particle_count",
            "particle_count",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// `ParticleEngine.clearParticles`: every particle and emitter goes before the next
    /// frame is drawn or the next particle tick runs, whichever comes first. It takes
    /// `&self`, so a screen or render hook can ask, as with `native_play_local_sound`.
    /// False on a server.
    fn clear_particles(&self) -> bool {
        crate::phase::unprovided(
            "ClientHost::clear_particles",
            "clear_particles",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The open menu's title (the block's name, or the one it was given),
    /// as plain text.
    fn screen_title(&self) -> Option<String> {
        crate::phase::unprovided(
            "ClientHost::screen_title",
            "screen_title",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// Where the mouse is, in GUI pixels.
    fn cursor(&self) -> [f32; 2] {
        crate::phase::unprovided(
            "ClientHost::cursor",
            "cursor",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The saved key and modifiers for an owner-bound registration, or its declared default.
    fn mod_key(&self, mod_id: &str, name: &str) -> Option<(String, ClientModifiers)> {
        crate::phase::unprovided(
            "ClientHost::mod_key",
            "mod_key",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// Shows the panel beside container screens with these items (at most
    /// 8192), as the game's own slots in pages, with a search box; the
    /// player's clicks, searches and keys over items come back as
    /// `native_panel_event`. The panel is the last mod's to show one; empty
    /// items and no recipe page take yours away.
    fn panel_items(&mut self, mod_id: &str, items: Vec<NativeStack>) {
        crate::phase::unprovided(
            "ClientHost::panel_items",
            "panel_items",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Places a recipe in the open screen's grid as a click in the recipe
    /// book does (`ServerboundPlaceRecipePacket`): the inventory's grid or a
    /// crafting table's. `all` places as many as the items allow, as with
    /// Shift. Refused while no such screen is open and for a recipe the
    /// player's book doesn't hold; one missing items shows as a ghost.
    fn place_recipe(&mut self, book_id: i32, all: bool) -> Result<(), String> {
        crate::phase::unprovided(
            "ClientHost::place_recipe",
            "place_recipe",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Creative mode only: gives the player `count` of `item`
    /// (`minecraft:diamond`) as taking it from the creative inventory does
    /// (`ServerboundSetCreativeModeSlot`): the selected slot first, then the
    /// hotbar, then the inventory. The count is cut to one stack.
    fn give(&mut self, item: &str, count: u32) -> Result<(), String> {
        crate::phase::unprovided(
            "ClientHost::give",
            "give",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// For a client mod: a push added to the local player's velocity, in blocks a tick, as an
    /// explosion's knockback is (the window's player moves itself, as vanilla's client does). Each component is
    /// clamped to -4..4; a spectator, or a value that is not a number, is left alone.
    fn push_player(&mut self, kick: [f32; 3]) {
        crate::phase::unprovided(
            "ClientHost::push_player",
            "push_player",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// The client half: sends `data` on `channel` to the server (the same
    /// caps, 32 a tick). An error when the server doesn't carry mod messages
    /// (a vanilla server), and until the player has joined: a server says it
    /// carries them during configuration, before it can take one.
    fn net_send_to_server(&mut self, channel: &str, data: &[u8]) -> Result<(), String> {
        crate::phase::unprovided(
            "ClientHost::net_send_to_server",
            "net_send_to_server",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// A mod's own RGBA image, uploaded into the HUD atlas slot
    /// `MOD_IMAGE_SPRITE` and drawn with
    /// `native_sprite`: its own source rectangle crops and stretches it. `rgba`
    /// is `width * height` pixels of four bytes, each side at most
    /// `MOD_IMAGE_SIZE`; false when it is not that
    /// size. The upload costs a texture patch of the image, so send one when the
    /// image changed, not every frame.
    fn mod_image(&mut self, width: u32, height: u32, rgba: &[u8]) -> bool {
        crate::phase::unprovided(
            "ClientHost::mod_image",
            "mod_image",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `base` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait BaseBackend {
    /// Side: server.
    /// The IP address the player connects from, without the port (behind a
    /// Velocity proxy, the address the proxy forwards). Displaced connected
    /// players keep their address; an acting player with no known connection
    /// has an empty address. None for a stale or unavailable player.
    fn player_address(&self, player: PlayerRef) -> Option<String> {
        crate::phase::unprovided(
            "Backend::player_address",
            "player_address",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The stable name of a connected or acting player. None for a stale or
    /// unavailable player; displaced online players retain their identity.
    fn player_name(&self, player: PlayerRef) -> Option<String> {
        crate::phase::unprovided(
            "Backend::player_name",
            "player_name",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `base` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait BaseBackendMut {
    /// Side: server.
    /// A system line in that player's chat, also inside a player's hook for
    /// the acting player; false when they aren't on.
    fn player_tell(&mut self, player: PlayerRef, line: Text) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_tell",
            "player_tell",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// A line over that player's hotbar (the action bar), also inside a
    /// player's hook for the acting player; false when they aren't on.
    fn player_action_bar(&mut self, player: PlayerRef, line: Text) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_action_bar",
            "player_action_bar",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The lines over and under that player's list of players (Tab):
    /// `ClientboundTabListPacket`, an empty text for none. False when they
    /// aren't on.
    fn player_tab_list(
        &mut self,
        player: PlayerRef,
        header: &Text,
        footer: &Text,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_tab_list",
            "player_tab_list",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether that player counts for `players_sleeping_percentage`: an
    /// ignored player who isn't in bed is left out of the players the night
    /// needs asleep, as a spectator is (an away-from-keyboard mod). Kept
    /// until a mod sets it back or the server stops, also if they leave and
    /// come back. False when the server list is unavailable, the handle is
    /// stale or the player isn't online.
    fn player_set_sleep_ignored(&mut self, player: PlayerRef, ignored: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_sleep_ignored",
            "player_set_sleep_ignored",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Shows `nickname` in place of that player's name in chat (their
    /// messages, `/me`, `/msg`) and in the Tab list; None shows their name
    /// again. Hovering the shown name gives the real one, which stays in the
    /// log, commands, selectors and the server's events. Kept until a mod
    /// sets it again or the server stops (also if they leave and come back).
    /// Plain text: the mod checks it (`Messages::text_allowed`, its length,
    /// other players' names). False when the server list is unavailable,
    /// the handle is stale or the player isn't online.
    fn player_set_nickname(
        &mut self,
        player: PlayerRef,
        nickname: Option<&str>,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_nickname",
            "player_set_nickname",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Hides that player (a vanish) from the players below permission
    /// `level`, or shows them again (None): from the next tick those players
    /// lose them from the Tab list and the world (their client stops
    /// tracking the player, as it does a spectator), and don't read their
    /// joining or leaving; players of `level` or more still see them, and
    /// a player made an operator meanwhile sees them again. Kept by UUID, even
    /// after this handle disconnects (the player rejoins hidden), until a mod sets it again
    /// or the server stops. Works inside a player's hook too; false for a stale
    /// world epoch, on a client and in a world nobody else can join.
    fn player_hide(&mut self, player: PlayerRef, level: Option<u8>) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_hide",
            "player_hide",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
