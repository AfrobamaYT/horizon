A wasm mod for `tests/points.rs`, built by `cargo horizon-mod build --wasm` (CI's wasm-mods job):

- `chat_tail`: listens at `handle_chat_input` and appends `W` to the message, so a test can see
  where it runs among a point's listeners.
