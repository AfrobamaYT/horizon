# Fake players acting each tick (`PlayerRef::perform_action`)

`game::native_server::fake_action_tests::fakes_acting_each_tick_cost`: 1, 64 and 256 fake players, each clicking
the stone floor with an empty hand once a tick (`PlayerAction::UseOn`, the full admission and use path; vanilla
passes), 200 ticks, release build:

```sh
HORIZON_TEST_RESOURCES=.local/horizon-client/26.3 cargo test --release -p horizon-client --lib \
  fakes_acting_each_tick_cost -- --ignored --nocapture
```

AMD Ryzen 7 5700X, Linux, 2026-11-04, at `29a6b3e28`:

| Fakes | A tick | An action |
|---|---|---|
| 1 | 3.9 µs | 3.9 µs |
| 64 | 280 µs | 4.4 µs |
| 256 | 1.56 ms | 6.1 µs |

256 fakes acting every tick take about 3% of a 50 ms tick. An action costs more as the fakes grow in number: each
one finds its guest by a scan of the level's guests (`perform_action`, `as_guest` and the handle read), so a tick of
N actions is O(N²) in that lookup. Nothing runs when no mod calls `perform_action`.
