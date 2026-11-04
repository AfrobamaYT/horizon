# {id}

A Horizon native mod. `horizon-mod.toml` is the manifest; `src/lib.rs` implements `Hooks`.

Build it with `horizon-launcher native-build . --instance <instance> --trust-native-code`.

Need something no hook offers? Follow the injection ladder in the modding book
([`docs/modding/src/injections.md`](https://github.com/arismoko/horizon/blob/main/docs/modding/src/injections.md)):
a hook or Part method first, then a `trait_method` injection with a `Host` binding, and a `helper` bound to
`Game` only as a last resort. Every injection is a hook request.
