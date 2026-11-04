# The mod API (rustdoc)

The reference for every hook, event, handle and `Host` method is the rustdoc of the `horizon-mod-api` crate
(lib name `mod_api`), generated from the `///` docs of the declarations in `hooks/*.rs`:

```sh
cargo doc -p horizon-mod-api --no-deps --open
```

Each generated `Hooks` method, event and `Host` method carries its `doc`, with the side and cancel contract the
catalog records. CI builds it with `-D warnings`, so a broken intra-doc link fails. The per-hook prose that predates
the `doc` field is in [Hook notes](hook-notes.md) until it moves into the fragments.
