# Sync 28 integration decisions

Target: horizon-rust/horizon at `d9b9996a4dfc61d0316daf599d7f937ccb8b95a8`.
Fork base: `5724caf147`. Common ancestor: `501446443c`.
Upstream changed 659 paths; the initial merge produced 204 conflicted paths.

## CI ownership

Keep all fork-base `.github/workflows/` and `tools/ci/` files. Upstream's runner and parity policy changes do not replace this fork's Dyna checks, static verification, runtime leases or release configuration. Retain the fork-specific parts of CI runner documentation. No CI dispatch is performed while the rollup gate holds runners.

## Launcher uninstall and identity

Take upstream's moved-data uninstall handling and its build-directory ownership guard. The default data-directory ownership check continues to use the fork's complete identity namespace rather than upstream's generic `horizon-launcher` leaf. The incoming default-path test uses the configured launcher identity so it applies to both builds. Moved locations keep upstream's `launcher`, `game` and `build` layout, and never delete the game directory.

## Developer tools and documentation

Combine upstream's optional `--harnesses` self-check flag with the fork's exact revision and dirty-worktree reporting. Preserve fork executable names in the controller harness instructions. Take upstream's new on-screen keyboard cases without changing their assertions. Layer documentation takes upstream's page form API but retains the fork dependency inventory rather than replacing it with upstream's exception count. Layer allowances preserve fork dependencies alongside incoming telemetry classification.

## Port ledger and translations

Merge port-ledger rows by Java method key: unchanged upstream rows retain the fork's relocated paths; changed upstream contracts replace the matching row, with unambiguous source paths relocated to the fork's game-area layout. Merge Thai launcher translations by key, taking upstream changes while retaining fork-only keys. Gameplay and rendering translations are resolved in their owning areas.


## Engine, item and server integration

Audited all 22 assigned paths and the additional random.rs correction. Two relocated destinations are also changed: crates/horizon-server/src/lib.rs and crates/horizon-server-launch/src/config.rs. Total: 25 paths audited, 23 changed, 2 deliberately unchanged. No unresolved textual cases or requested rulings.

- Item model tests retain both the registered mod node test and upstream linear/binary threshold coverage.
- The three language files retain fork keys and take upstream added/changed translations, including Horizon UI text.
- NBT: upstream changed its old horizon-world/src/nbt.rs, now relocated to horizon-core. Ported Cursor/Key and entry/list header primitives there, preserving fork heap quotas, bounded reads, strict duplicate handling, split recursive frames, and list_fits validation. Cursor and Key must be public because structure template decoding is now in a separate crate. No obsolete world NBT file restored.
- Items retain explicit DAMAGE presence and include_damage gating. Upstream prototype-aware removal of equal DAMAGE 0 is integrated at serialization. Upstream custom durability/mining modules, tag replacement, weapon blocking, and consume parsing integrate with existing fork modules. The duplicate Items attribute_modifiers implementation is not introduced: the fork's ItemStack::default_attribute_modifiers already owns that behavior and incoming tests already call it.
- Protocol registry.rs remains unchanged from scaffold: fork registry count limits and registry-scoped tag replacement retained; upstream conflicting tag changes are documentation/parameter naming, not a missing behavior.
- Server files expose both durable and incoming chat_guard.
- Server backup security, pruning, rollback on failed restore placement, and upstream tests integrated. Interval UTF-8 and overflow corrections and tests ported to the relocated Backups::interval in server-launch/config.rs, rather than restoring duplicate backup::interval.
- Server engine retains server-launch command construction; upstream parallel_tick is forwarded by environment without duplicating launcher logic. Control line-break rejection and its incoming regression test integrated. Parallel execution itself remains game-b's single disabled gate responsibility.
- Server main.rs remains the thin library wrapper. Incoming signals::stopped cleanup is applied in relocated server/lib.rs instead. Supervisor retains launch helpers and takes upstream queued restore/update handling before every start.
- Piston events keep note_override and take upstream queue/pop deduplication routing. Existing upstream level redstone and brightness tests included.
- Structure changes take upstream streaming template decoding and saved legacy piece reads/tests. Imports use relocated horizon-core NBT/BoundingBox and retain fork block entity transform module. Saved-template int/flag use the new shared saved_legacy helpers rather than duplicate implementations.
- LegacyRandom state/gaussian conversion initializes gate to None, matching independent clone/from_state behavior. No Mob RNG API types changed here.

Verification: rustfmt edition 2024 with skip_children=true on touched Rust files; git diff --check passed. No builds, tests, clippy or pushes performed. Patch is binary-capable and based directly on scaffold. Compilation and CI remain coordinator-owned.

## Clean-merge correction and module placement

The native explosion state merge saved blast ownership twice, then restored empty duplicate values over the outer blast attribution. Remove the duplicate saves and restores, retaining the incoming player-owner flag and the fork nested-blast contract. New extinguish, frame autotuning and smithing icon modules move into their existing entities, runtime and interface areas through the committed relocation script.

## Parallel mob dispatch: authorized performance divergence

Diverges from upstream (performance only). Root authorizes serial dispatch while preserving upstream parallel implementation behind one fork-owned gate, with minimal typed adapters where required. Exact gate and adapter inventory follows the gameplay resolution. Upstream horizon-rust/horizon#15493 states “Not yet shown to give a CPU gain.” The originating horizon-rust/horizon#15056 states “This draft does not yet establish a worthwhile performance gain or acceptable fallback overhead.” No upstream measured gain is claimed here. Tests and serial behavior remain unverified locally.

## client resolution details

## Contracts and structural rulings

- Preserve JoinList and its validated mutation boundaries, typed hook contexts, fork Parts vocabulary and Wire API. Do not replace them with upstream MobSlot slices or contextless services.
- Preserve Mob.random, Mob::with_random, mobs::entity_random and living_random as LegacyRandom. New PlayerMotion.random is EntityRandom, constructed with EntityRandom::own(mobs::entity_random(...)) in the existing player_init destination. World.random stays LevelRandom (engine ownership); no world RNG reversion.
- Projectile owns exactly one random stream in Projectile.random: OwnRandom(LegacyRandom). Constructor consumes UUID's first two draws from that stream; shoot_own advances the same stream used by arrow ticking. ArrowState::new() stays unchanged; do not introduce ArrowState.random. shoot_from_rotation uses upstream's no-RNG-argument signature. with_entity_random still overrides the owned stream for fixtures.
- Preserve typed blocking damage and shield_damage. Add blocking_item_used through Environment, PausedHurt and BlockOutcome, initialize false, copy across suspended equipment wear and set when blocking processing runs. The removed health/equipment.rs is not restored.
- Upstream external renderer seam, linear-light constants, dimension-aware rendering, chat guard interface, far-ring fairness, banner tag support, item consumables and NBT narrowing are integrated with fork APIs rather than importing obsolete source layouts.
- Keep fork relocated implementations instead of adding duplicate upstream extraction modules: renderer census/frustum helpers, player_init, options layout/presets with native controls, mob seen/pending pushes, entity NBT armor stands/rockets/orbs. Incoming behavior is integrated at the live implementation. PendingPushes additionally gets Clone and append for upstream helper code.
- Preserve the fork asynchronous screenshot writer and its typed hooks rather than replace it with upstream's different write-completion API. Runtime window_handler adopts frame_autotune for dynamic resolution while retaining typed pause hooks. Existing trace/span logging remains there.
- world_warp.rs and world_warp_fragment.wgsl remain deleted; their renderer-side warp replacement machinery was already removed by the typed render architecture. No old warp test or mod service is restored. Inter-stage coverage takes the new ray-unpack test only.

## Path audit

### Renderer

- Cargo.toml: retain mod dependencies; take removal of Horizon cosmetics dependency from the engine layer.
- block_entities.rs: merge sign editor tracking with attachment generations and observers.
- gfx.rs: external pipeline/frame policy/linear light integration; keep prepared model and render hooks; shared entity factory accepts linear-light mode; preserve primitive reserve and census helpers.
- gfx/inter_stage_tests.rs: new ray-unpack coverage, no deleted world warp tests.
- gfx/world_primitives.rs: linear frame shader contract plus fork stage/capacity contract.
- gfx/world_warp.rs and gfx/world_warp_fragment.wgsl: keep deleted, as above.
- hud.rs and hud/pack_text.rs: combine model-reducing and menu picture modules; split GUI/world text shaders while retaining ResourceSource.
- options_ui.rs: take chat-delay and pack-drop behavior and widget visibility; retain native settings and existing extracted layout; preserve native slider events and use upstream four-stop preset mapping.
- packs.rs: copy_pack export and case-insensitive zip detection; retain resolved source contract.
- particles.rs: camera quaternion rendering plus retained starter/ordinary pending queue and resource source test; add quad rotation test using indexed source.
- particles/polytone.rs: upstream particle behavior with fork pending enum lookup.

### Client systems

- banner.rs: merge pattern tags with fork ItemDefaults loading; keep item metadata ownership.
- client_lib.rs: expose parallel and dynamic_resolution, retaining block_sounds and fork mod/render reexports.
- entities.rs: obstacle/water-splash behavior and entity data changes, preserving context-bearing fluid updates and before-velocity splash input.
- entity_nbt.rs: upstream byte/short codec behavior, horse variants and regression tests; preserve full typed equipment and attachment/join initialization. Keep inline fork armor stand/orb/rocket implementations rather than duplicate imports. Use tame restore_sitting.
- entity_nbt/hanging.rs: loaded pitch and byte-wrapped facing; retained item-frame implementation in parent uses those helpers.
- entity_nbt/projectile_fields.rs: projectile-kind codec behavior with fork horizon_core NBT import.
- entry.rs: install upstream chat filter; retain fork horizon_look path.
- game.rs: baseline dimension type/environment/render changes and tick spans; preserve typed harvest/critical/sweep/orb hooks. Includes game-a-game-rs.patch layered cleanly over baseline. Coordinator must not double-apply that standalone game.rs delta.
- health.rs and hunger.rs: blocking_item_used propagation, typed damage retained.
- health/equipment.rs: stays deleted; its field propagation is in active health.rs.
- item_sync.rs: consumable codecs and imports, retaining fork attribute/kinetic modules and NBT layer.
- item_tooltip.rs: upstream item display behavior with typed attribute search context retained.
- lan/hub.rs: upstream chat guard interface, air-state table checks, duplicate teleport refusal, teleport-id invalidation, far-ring scan cursor and missing-column retries; preserve JoinList updates, watch notifications, snapshots and typed effect particles. sync_data accessor added.
- lan/hub/chat_filter.rs: import server-files guard contract and Place; no direct engine-owned Horizon filter guard.
- lan/hub_tests.rs: stays deleted. New cave-air/far tests are wired into active inline hub tests; duplicate-accept regression moved there.
- lan/io_pool.rs: memory-connection cap exemption; obsolete fork-removed packet-tally tests not restored.
- menu.rs and menu/place.rs: quick-move take helper and placement behavior combined with fork typed inventory inputs.
- mobs.rs: retain JoinList/damage/target contracts and rich StoredEffect values; port client-side damage guard, visibility/foreign-seen data, random ownership boundary, tame pose and shared model extraction. Keep fork inline seen/pushes implementations; append/Clone support added.
- mobs/ai.rs: camel age, fire reset/powder-snow behavior with existing typed air/damage/consumption pipeline; do not restore duplicate extracted walk-target/contact tests or contextless attack routine.
- mobs/aquatic.rs: unchanged after audit; upstream delta only introduces generic MobSlot access, refused in favor of JoinList validation.
- mobs/conversion.rs: preserve full typed StoredEffect/absorption conversion, including icon copying; take unrelated upstream deltas.
- mobs/enderman.rs: upstream behavior around existing typed target approval rather than direct target assignment.
- mobs/equine.rs: horse variant codecs and filtered bolt destination goal; retain typed test level.
- mobs/living.rs: drowned current immunity and boat fluid interaction state; preserve active fork movement hooks.
- mobs/piglin.rs: incoming behavior with typed target proposals and concrete player identity retained.
- mobs/riders.rs: boat/fluid behavior; preserve JoinList navigation loan and paused-consumption restoration rather than MobSlot ticking.
- mobs/spawning.rs: runtime finalize regressions adapted to StoredEffect values and typed helper.
- mobs/spear_use.rs: unchanged; upstream's event change is contextless player-event routing that the fork already replaces with typed routing.
- mobs/turtle.rs: unchanged after audit; retain typed TestLevel and LegacyRandom rather than incoming generic RNG test rewrite.
- mobs/wet_fire_tests.rs and mobs/zombie_villager.rs: upstream behavior/tests adapted to retained Mob LegacyRandom and profession data.
- motion.rs: retain player_init instead of duplicate body module; add entity-owned player random through existing initializer.
- multiplayer.rs and title.rs: upstream title entry/field handling and world-folder naming with fork screen transitions retained.
- projectiles.rs: unified RNG/UUID contract described above; rotation/fire/below-world changes retained; no contextless block-contact service added.
- save.rs: upstream save backpressure plus fork ticket serialization/fluid interning.
- vanillagen/structure_mobs.rs: background load owns Arc<ItemData> and uses its defaults; retain contextual fork cache initialization.

## Destinations outside original list

- crates/horizon-client/src/motion/player_init.rs: initialize new entity-owned player RNG without duplicating initializer.
- crates/horizon-client/src/game/runtime/window_handler.rs: upstream frame_autotune invocation at fork relocated frame completion point.

## Verification and remaining gate

Ran rustfmt, edition 2024, skip_children=true, on changed Rust paths; no formatting errors. git diff --check is clean. No builds, tests, clippy, pushes or PR actions were run, as requested. Compilation and behavioral correctness remain for coordinator compile integration and CI; this document does not claim either passed. No known textual conflict markers or design rulings remain.

## game-a resolution details

# Sync 28 game-a integration

All 64 assigned paths audited; final patch changes 56 destination files. Local commits: 2e050d671e5, f700ea8404f and fe14b136bd7. Patch base is scaffold 39a01b166e252ce2f0e8dd34d66c2bfdcede09df. No pushes, builds, tests or clippy were run.

## Verification

All changed Rust destinations were formatted with edition 2024 and skip_children=true. git diff --check against scaffold passes. Working tree is clean. Compilation and tests are NOT checked; coordinator owns the compile-only gate.

## Integration decisions

- Deleted root gameplay paths stay deleted. Their changes land in game.rs, blocks, items, mobs, network, runtime, world, entities and modding destinations, not restored obsolete modules.
- Retain JoinList update closures, residency extraction, validated guest turns, scoped invulnerability checks, typed damage/attack/victim identity, typed spawning, Parts layout and Wire contracts.
- Incoming projectile own-RNG shooting retains spawn_projectile rather than restoring add_projectile. Incoming parallel item moves are wired through game.rs's ahead module, with per-turn saved invulnerability ticking retained and the existing native-hook refusal in ahead preserved.
- Random tick chunks are sorted deterministically, with masks prepared before ticking; tests are explicitly wired from game/block_ticks/random_order_tests.rs.
- Item void bounds use the live level minimum. Existing fork escape tests retain independent seeded expected values and ticking-neighborhood preparation rather than replacing them with implementation-derived expectations.
- Guest body initialization remains inline in the validated JoinList turn, not upstream's unchecked mutable-Guest extraction. Its own entity RNG is initialized before saved data is applied. Drop resynchronization runs through self.as_guest and updates the guest's told-selected state afterward.
- Merchant refresh signaling and OffersRefreshed detection retain fork-owned LAN borrowing signatures. Reputation changes refresh prices; ordinary takes no longer resend offers unnecessarily.
- Keep fork damage_stack, harvest/break-speed hook, tooltip hooks and native ram combat adaptations instead of restoring upstream's obsolete implementations. The existing fork helper already covers mob_pushes_player with explicit attack context.
- melee_damage_with_admission keeps victim: Option<u32> as a third argument, alongside the old typed melee_damage wrapper, so damage RNG is evaluated once while exposing admission.
- Preserve ItemConsumable's synchronized prototype/data/wire lookup through Game::consumable while keeping typed can_eat and hook calls. Borrow the returned owned consumable for completion.
- Keep sync 27's refusal to restore entity_portal_delay and contextless native RNG/entity services. Take live portal cooldown serialization; dimension scaling now uses DimensionFacts.
- Keep the existing waypoint Icon vocabulary; persistence is implemented around it, without introducing a duplicate WaypointIcon type.
- Take upstream Horizon cosmetics settings, PvP helpers, dedicated crash-stop behavior, beacon text, sign edit lifecycle, crack sprites and world-update spans. Existing fork save-before-unload hooks remain in their established order.
- New position_args validation lands at administration/commands/position_args.rs, cosmetics settings at interface/cosmetics/settings.rs, and renamed upstream extras/pvp.rs merges into entities/pvp.rs. No duplicate extras/pvp.rs is restored.
- World content access retains server_items precedence while allowing remote.tables.items fallback; item_enchantments reads remote.tables.enchantments.
- smithing_slot_icons is imported from the logical game::smithing_icons module; coordinator is moving that module into interface.

## Shared-file ordering

The full patch includes game.rs changes from assigned deleted paths. game-a-game-rs.patch isolates that delta. sync28-client owns the baseline game.rs conflict resolution and must layer these changes without losing its baseline resolution. game-b was told to exclude entities/pvp.rs because this patch owns it. game-b was told the three-argument melee admission signature and that nether_lighting, pvp_mutes_sound and stop_after_crash are supplied here.

## Audit artifacts

The JSON and conflict-review text were saved at the initial checkpoint for traceability. Their UNMATCHED and CONFLICT labels describe placement attempts at that checkpoint, NOT the final status: a second function-level and manual semantic pass integrated or retained the fork-equivalent contracts above. They are not a deferred-work inventory.

No design rulings are requested. Remaining integration risk is type checking and overlapping game.rs patch application; neither was locally compiled, as instructed.

Shield-use statistics additionally require the incoming health::BlockOutcome.blocking_item_used field; client worker and coordinator were notified explicitly. Shield wear remains fork-owned damage_stack.

## game-b resolution details

## Diverges from upstream (performance only)

Root ruling 2 is applied. Single fork gate: `crates/horizon-client/src/parallel.rs::parallel_tick`, local constant `FORK_PARALLEL_TICK_ENABLED = false`. It disables ALL opt-in parallel server phases, not only mob phases. The upstream pool, par_map and verification machinery otherwise remain in the tree.

Exact type blockers: `mobs::Mob.extra.live` is `living_attributes::LivingAttributes`, whose map is `std::cell::OnceCell<Box<Live>>`. `Live = LiveAttributes<Box<dyn Identities>>`; `Identities: FnMut() -> i32` lacks Send and Sync bounds. `LiveAttributes::data` owns that allocator callback. OnceCell -> OnceLock alone therefore cannot make Mob Send + Sync; no unsafe shim or cache refactor was taken. The fork tick additionally owns `JoinList<Mob>` and `health::LentLevel` (LegacyRandom, attachments and syncs), not upstream plain Vec tick ownership. Smallest active adapters remain `BorrowedMobTick` and extracted serial `fill_mob_state`, reading the existing authoritative live attribute map.

Upstream `game/creatures/{mob_group,mob_islands,prep}.rs` are retained BYTE-IDENTICAL at upstream paths. Only their module declarations are behind nondefault Cargo feature `upstream-parallel-mobs`; no default enables it. **This code is not compiled or tested on the fork.** Do not turn that source-retention feature on to claim working parallel dispatch. No currently compiled fork code calls those modules. Their eventual typed adaptation is fork #3491.


## RNG adapters

World swaps use `LevelRandom::for_level(LegacyRandom::new(...))`. Hook-owned LentLevel continues to expose LegacyRandom via the world owner's deref adapter, coordinated by parent/client. Mob.random and mobs::entity_random remain LegacyRandom by client ruling. Projectile shoot_from_rotation uses upstream no-caller-RNG signature; own_uuid/uuid are upstream projectile contracts expected from engine/client integration. No RNG draws added by wrappers.

## Path audit

| Assigned path | Resolution |
| --- | --- |
| `crates/horizon-client/src/game/entities/armor_stands.rs` | Upstream special-hit admission/tail and dimension-type lighting; victim identity remains typed. |
| `crates/horizon-client/src/game/entities/boats/below_world.rs` | Created at assigned relocated destination; validated JoinList removals and below-world parity regressions. |
| `crates/horizon-client/src/game/entities/boats/mod.rs` | Float-exact paddle/escape vectors, freeze counter, last-Y movement and below-world behavior; typed held-out mutation retained. |
| `crates/horizon-client/src/game/entities/entity.rs` | Kept fork loop-context block-effects implementation instead of upstream hookless ItemTables move/settle extraction; added item air fixture field. |
| `crates/horizon-client/src/game/entities/explosion.rs` | Explicit explosion source/credit behavior merged through DamageLevel; upstream regression uses typed LevelRef explosion boundary. |
| `crates/horizon-client/src/game/entities/frames.rs` | Projectile frame hits, creative-owner drops and dimension lighting; typed hook rotation remains normalized. |
| `crates/horizon-client/src/game/entities/minecarts.rs` | Freeze state, below-world removal, exact control yaw; moved typed attack into special_attacks; kept live ownership and NBT freeze persistence. |
| `crates/horizon-client/src/game/entities/pvp.rs` | Audited upstream rename to extras/pvp.rs; final content supplied by game-a by coordinator agreement, excluded from this patch. |
| `crates/horizon-client/src/game/mobs/creatures/mob_group.rs` | Byte-identical upstream source retained at original upstream path behind nondefault source-retention feature; not compiled. |
| `crates/horizon-client/src/game/mobs/creatures/mob_islands.rs` | Byte-identical upstream source retained at original upstream path behind nondefault source-retention feature; not compiled. |
| `crates/horizon-client/src/game/mobs/creatures/mod.rs` | Kept JoinList lift/restore + BorrowedMobTick, ported observable event/ignition/projectile/UUID behavior and caller RNG signature. Source-retention declarations isolate incompatible parallel extraction. |
| `crates/horizon-client/src/game/mobs/creatures/prep.rs` | Byte-identical upstream source retained at original upstream path behind nondefault source-retention feature; not compiled. |
| `crates/horizon-client/src/game/mobs/creatures/projectile_targets.rs` | Created relocated active pickable/projectile-target implementation, including primed TNT and item frames; retained separate fork vehicle targets for typed callers. |
| `crates/horizon-client/src/game/mobs/creatures/projectile_world.rs` | Kept fork context-owned block-contact path; ported projectile world floor query. |
| `crates/horizon-client/src/game/mobs/dragon.rs` | Cloud velocity and target computation updated; moved typed special attack to items/special_attacks. |
| `crates/horizon-client/src/game/mobs/enderman.rs` | Placement survival and non-mob occupancy integrated around existing typed leave-block decision/apply transaction. |
| `crates/horizon-client/src/game/mobs/hoglin_repeat_attack_tests.rs` | Retained typed target proposal contract and appended two upstream broadcast distance regressions with source-player connection adaptation. |
| `crates/horizon-client/src/game/mobs/mounts.rs` | Upstream public escape vector; Mob RNG remains LegacyRandom by client ruling. |
| `crates/horizon-client/src/game/mobs/ominous_item_spawner/mod.rs` | Loaded rotation normalization uses fork core NBT types and mutable owned input. |
| `crates/horizon-client/src/game/mobs/raids/mod.rs` | World owner RNG swaps wrap LegacyRandom in LevelRandom; scoped patrol draws preserve stream. |
| `crates/horizon-client/src/game/mobs/spawner.rs` | World owner RNG swaps use LevelRandom::for_level; no extra draw. |
| `crates/horizon-client/src/game/network/lan/mob_states.rs` | Active relocated serial fill_mob_state adapter reads typed live attributes. Upstream parallel Later/AttrSources requires Sync Mob and is not used. |
| `crates/horizon-client/src/game/network/lan/mod.rs` | Metadata uses upstream sitting pose while preserving typed effect-particle modifications and live attributes; serial state helper extracted; broken stat added. |
| `crates/horizon-client/src/game/network/lan_actions/tests/guest_body.rs` | Upstream extraction relocated to production lan_actions/guest_body.rs, not obsolete tests location. Constructor failure still refuses turn before state loan; writes through Guest JoinList::update. |
| `crates/horizon-client/src/game/network/lan_actions/tests/guest_body/tests.rs` | Upstream saved-body/distinct-stream regression retained at production helper test destination. |
| `crates/horizon-client/src/game/network/lan_attributes.rs` | Fork live-map metadata kept as source of truth; upstream equipment-derived parallel AttrSources would duplicate components and does not compile against !Sync Mob. |
| `crates/horizon-client/src/game/network/lan_mobs.rs` | Preserved fork typed hoglin target transaction; ported Ignite/ZombieIgnite routing. |
| `crates/horizon-client/src/game/network/lan_vehicles.rs` | Preserved Guest JoinList::update validation and typed host-borrow contract; no obsolete mutable guest indexing. |
| `crates/horizon-client/src/game/network/remote/mod.rs` | Kept typed tag notices/residency; ported item-use/abilities/gameplay tag extraction, source sound and respawn attributes; removed duplicate UpdateTags dispatch. |
| `crates/horizon-client/src/game/network/remote_mob_data.rs` | Sitting pose metadata integrated. Existing typed villager receive already owns Enderman stare sound, so no second scratch-state implementation. |
| `crates/horizon-client/src/game/network/remote_vehicles.rs` | Client-side horse damage marker integrated with fork LentLevel tick. |
| `crates/horizon-client/src/game/world/end.rs` | Gateway delay phases use existing typed dimension_changing_delay instead of removed contextless entity helper; dimension-type environment/lighting kept. |
| `crates/horizon-client/src/game/world/integrated.rs` | Catch/save integrated tick failure merged while retaining crashed flag and typed close lifecycle. New regression uses fork test factory. |
| `crates/horizon-client/src/game/world/levels.rs` | Upstream per-level spans retained around typed enter_level and hook LevelRef bodies, draining pending inside-block events before post. |
| `crates/horizon-client/src/game/world/packs/mod.rs` | Upstream drop/import UI integrated with mod-discovered additional packs and caller update. |
| `crates/horizon-client/src/game/world/persist.rs` | Retained fork transferred-object initialization, passenger admission and typed object-tree persistence. Ported crystal fire cleanup, no-save leash knots, portal cooldown saves and camel reload regression; destination object_memory gains own projectile UUID and boat freeze state. |

## Destination ownership

Additional edited/moved destinations: parallel.rs and horizon-client/Cargo.toml (gate/retention feature); items/special_attacks.rs and special_attack_fixture.rs; network/remote/{abilities,item_use,tables}.rs; network/remote_tags.rs (keeps typed notices and refreshes gameplay tags); network/lan_actions/{mod,turns,guest_body}.rs plus guest_body/tests.rs; interface/options.rs import caller; blocks/nether_portal.rs dimension-delay visibility only; world/persist/object_memory.rs; mobs/creatures/frame_projectile_tests.rs; mobs/enderman_occupancy_tests.rs; network/pet_pose_meta_tests.rs; world/gateway_cooldown_tests.rs. Parent was notified of overlap; game-a supplies PvP content. Three game-a overlapping file intents were integrated: freeze NBT + seed reset in minecarts; seed reset + frame regression declaration in creatures; per-ID guest random in extracted initialize.

## Upstream behavioral tests retained and statically assessed

- `a_vehicle_falls_out_of_the_world_64_blocks_under_the_floor`, `a_tick_discards_the_vehicles_under_the_world`, `saved_deep_end_boat_loads_then_is_discarded_and_extreme_scans_terminate`: serial removal happens before motion, spills container contents without ENTITY_DIE; no cross-thread behavior required.
- `a_carried_block_is_not_placed_into_an_entity`: placement validates survival and non-mob occupancy inside the existing typed decision/apply path.
- `a_broadcast_player_is_compared_by_distance`, `a_guest_broadcast_is_placed_as_the_guest`: typed target proposal compares source/current player identities and commits after hook approval.
- `co_located_guests_own_distinct_streams_and_restore_the_saved_body`: validated Guest update initializes the body with per-ID stream before saved state restoration.
- `a_cat_on_a_chest_sits_for_the_other_players_too`: metadata uses sitting_pose, not order state.
- `flint_and_steel_use_on_ends_the_click`: fire prediction sits after fork hook, cooldown and adventure gates and terminates the click on PASS-excluding admission.
- `a_servers_block_and_item_tags_replace_the_games`: gameplay items refresh independently of mod presence; typed mod notices remain distinct.
- `gateway_counters_match_actual_vanilla_methods`: real game identity-based delay selection replaces removed Entity trait helper; arrow/pearl phase ordering retained.
- `integrated_tick_panic_saves_changed_world`: catch-and-save loop preserves shutdown error and typed session teardown.
- `explicit_sources_keep_names_without_inventing_kill_credit`: preserved through typed LevelRef explosion call, no contextless service added.
- Frame projectile regression module and existing camel reload assertions retained. Parallel-only structural tests inside the retained upstream sources are not compiled/tested, as stated above.

These are STATIC assessments, not pass claims. Coordinator compile and CI remain required. No unresolved design ruling remains after root ruling 2; excluded parallel execution is an explicitly tracked divergence, not claimed shipped capability.

## Rollup 46 main integration

Merge current main `b96005b7c9115c9061fa996e00b5b029a7fc0efc` before publishing. The two inventory conflicts keep main's Menu/State/touch structure while preserving upstream quick-move result accounting and viewer-aware recipe transfers. Craft results report the crafted copy minus leftover; merchant results report the moved stack. Transfer snapshots and drop truncation read State, and mutations call touch. Test callers pass the existing viewer argument explicitly.

## Native hook declarations

The incoming native_hooks.rs change dereferences the new level RNG wrapper inside an obsolete contextless service. The fork removed that service; no hook declaration or capability is added back. Existing typed host and RNG contracts remain. Wire, derive, hook catalog and the parts vocabulary layout are unchanged by this sync.
