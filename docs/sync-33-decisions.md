# Upstream sync 33

Upstream pin: `f24ee1accf1c816736e6e067939d04f3853f474e` (horizon-rust main with #15361, the performance capture platform); its push-to-main `ci.yml` run completed with success. Previous integrated upstream: `a21f8a3947e9fa6fdf47cbc0f656d6444a622e2a` (sync 31). Sync 32 (#3554, pin `ba91f75cb21`) merged while this one was in progress; fork main was merged in afterwards, taking sync 32's resolutions where both touched the same code.

Started from fork main `d6643239398`. Initial conflicts: 51 paths, 12 of them files the fork moved into area folders.

## Production ownership

- The perf platform lands whole: `horizon-perf`, `horizon-perf-format`, `horizon-perf-share`, `/profiler`, the sampler and heap sampling in `horizon-threads`, the client, server-thread and dedicated recorders, Open and Share. The dedicated server's report is upstream's capture rewrite; the fork's only change there, counting online guests (`online_count`), is kept.
- `/profiler` and the fork's `/modprofile` stay side by side, unchanged. Merging them is a follow-up, not part of this sync.
- Upstream's six new game modules go to areas by `tools/move_game_modules.py`'s rules: `lan_op`, `perf_link`, `perf_remote` to `network/`, `profile_capture` to `runtime/`, `profile_lines` to `interface/`, and `integrated/capture.rs` under `world/integrated/` with a `#[path]`.
- Moved files carry upstream's hunks at their new homes: work attribution (`set_chunk`, `set_entity`, `clear_work`) in block entity ticks (named per `TickedKind`), block ticks, item entities, natural spawning, fangs, fluid ticks and the borrowed mob tick (`start_mob`/`end_mob`); frame timing and `record_perf_frame` in `runtime/window_handler.rs`; the profiler's launch options in `game.rs`'s `Options`; the perf channel on the guest packet path and the client poll; `finish_profile` on shutdown.
- `horizon-threads` is portable as upstream ships it; `horizon-guest-stack`, its dependency with no dependencies of its own, joins the portable list. The test-allocation-count allocator keeps precedence over upstream's sampling allocator.
- The span switch (`tick_profile::want_spans`) returns for `HORIZON_SPIKES` and reports, as upstream uses it; the mod user (`SPANS_MOD`) is not restored, since the fork removed `native_spans`.

## Deliberate divergences

| Boundary | Decision |
| --- | --- |
| World warp | The fork removed upstream's warp pipelines; render conflicts keep the fork's draw calls and add only the GPU timing stages (`pass.stage`/`pass.split`). |
| Streaming update | The fork splits the update into plan and commit; upstream's laps before the split are left out, the later ones time `complete_commit`. |
| Chest-boat loot | Sync 32 (#3554) ported upstream's chest-boat loot changes; this merge takes its versions of `machines`, `boats` and `lan_actions/menus.rs` unchanged. |
| Tick timing | The fork removed `native_server_tick_timed`; the merged spans it took are no longer built. |
| PlayerSender, Outbox | Only upstream's `queued` counters are added; the fork's removal of the per-sender fake flag and the packet tally stands. |
| Game rules in captures | Read through `GameRules::get`, the fork's typed accessor. |

No harnesses or game runs; compile-only verification (clippy `-D warnings` on the touched crates with all targets, the layer check, the file-size check).
