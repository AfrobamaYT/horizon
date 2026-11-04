Wasm mods for `tests/groups.rs`, built by `cargo horizon-mod build --wasm` (CI's wasm-mods job):

- `counter_lib`: a library mod publishing the capability `test:counter` and its handler trait;
- `alpha` and `beta`: two mods depending on it, so linked into one group; alpha answers it;
- `gamma`: a mod in a group of its own, asking a capability of the same name with its own
  handler trait, which no provider of another group answers.
