## Nightly from arismoko/horizon

This is the modding fork of Horizon at [arismoko/horizon](https://github.com/arismoko/horizon), not an official upstream Horizon release. **The official Horizon launcher does not install these builds. There is no prebuilt launcher for this fork yet.**

Use a launcher built from this fork's source. With Rust and the platform's native build prerequisites installed:

```sh
git clone --depth 1 https://github.com/arismoko/horizon.git
cd horizon
cargo run --locked --release --manifest-path launcher/Cargo.toml --bin horizon-launcher -- install nightly
```

Follow the printed `prepare --version ...` command next. To open the launcher's interface, run the same Cargo command without `-- install nightly`. Signed Nightlies still require an account that owns Minecraft: Java Edition. For now, set `HORIZON_LAUNCHER_HOME` to a separate folder if you also use the official launcher, so their launcher data does not mix.

The game archives below are payloads for the launcher, not launcher installers. A Nightly can include selected pull requests before they merge; report bugs to this fork with the exact Nightly date and commit shown by the game.
