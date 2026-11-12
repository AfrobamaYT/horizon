# Your first content mod

A content mod adds blocks, items, recipes, loot, models and textures with data files alone. In this page you
make one block, see it in the game, and pack it as a `.hmod`.

You need Rust's `cargo` and a Horizon game (`horizon-client`) to run the mod in.

## 1. Install the mod tool

```sh
cargo install --locked --git https://github.com/arismoko/horizon cargo-horizon-mod
```

This adds the `cargo horizon-mod` command.

## 2. Make the project

```sh
cargo horizon-mod new glowing-blocks
cd glowing-blocks
```

The tool turns the folder name into the mod id (`glowing_blocks`) and writes a complete mod with one block:

```text
glowing-blocks/
├── horizon-mod.toml                                   # the manifest: id, name, version, api, capabilities
├── assets/glowing_blocks/
│   ├── blockstates/glowing_amethyst.json
│   ├── models/block/glowing_amethyst.json
│   ├── items/glowing_amethyst.json                    # the item model
│   └── lang/en_us.json                                # the block's name
└── data/
    ├── glowing_blocks/horizon/block/glowing_amethyst.json   # the block itself
    ├── glowing_blocks/loot_table/blocks/glowing_amethyst.json
    ├── glowing_blocks/recipe/glowing_amethyst.json
    └── minecraft/tags/block/mineable/pickaxe.json
```

The block declaration starts from a vanilla block and changes what differs:

```json
{
  "template": "minecraft:amethyst_block",
  "properties": { "hardness": 2.0, "light": 7 }
}
```

The manifest declares the capability `registries`: a mod needs it to add blocks, items, mob effects or entity
types to a world.

## 3. Run it

```sh
cargo horizon-mod run
```

This builds `target/horizon-mod/glowing_blocks.hmod` and starts the game with the mod always on. It finds the
game on your `PATH`, or through `--game <path>` or the `HORIZON_CLIENT` variable; arguments after `--` go to the
game. Craft the block from an amethyst block and glowstone dust.

The game reads content once, at start-up: after you edit a file, run the command again.

## 4. Check and pack it

```sh
cargo horizon-mod build
cargo horizon-mod check target/horizon-mod/glowing_blocks.hmod
```

`build` validates the content with the same checks the game uses and writes the `.hmod`. `check` runs what the
mod registry runs on a submission. The `.hmod` is the file you share: players put it in `horizon-mods/`
([Installing and using mods](installing.md)).

## Next

- [Content mods](../how-to/content-mods.md): every kind of content file, from blocks to world generation.
- [The manifest and the package](../reference/manifest.md): every manifest field and package rule.
- [Publishing your mod](../how-to/publish.md): signing and the registry.
