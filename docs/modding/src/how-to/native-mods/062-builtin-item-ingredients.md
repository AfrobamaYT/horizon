<!-- chapter: Built-in item ingredients -->

# Built-in item ingredients

Item ingredients accept item ids, lists and `#item_tag` references. Ingredient objects
read `neoforge:ingredient_type` first, then `type`. The built-in NeoForge kinds are
`compound` (any child), `intersection` (every child), `difference` (base without
subtracted), `components`, `block_tag` and `custom_display`. These built-ins never
call a mod's `ingredient` hook. Holder-set objects are a separate format and are
not accepted here. These object forms are read from recipe resources. The
`Host::recipe_ingredient` convenience reader accepts flat item inputs.

```json
{"type":"neoforge:components","items":"minecraft:diamond_sword","components":{"minecraft:damage":5},"strict":false}
```

`components.items` accepts an id, list or `#item_tag`. Every component in the patch
must equal the concrete stack's effective value, including item defaults. A `!`
prefix requires absence: `{"!minecraft:enchantments":{}}`. `strict` defaults to
false; when true, every effective component must also be named in the patch.
For example, a diamond sword's default `minecraft:max_damage` counts. Count does
not participate; empty stacks never match.

```json
{"type":"neoforge:block_tag","tag":"minecraft:logs"}
{"type":"neoforge:custom_display","base":"minecraft:apple","display":{"type":"minecraft:item","item":"minecraft:diamond"}}
```

`block_tag.tag` names a block tag without `#`, matching its blocks' items.
`custom_display` matches its base and uses the slot display only for presentation.
As with packet slot displays, potion, dye and trim demo displays show their source
stacks. Matching, placement and network ingredient membership retain the base's
item list.

Plain item lists, item tags and block tags are simple. Components and mod kinds
are not simple. Compound, intersection and difference are simple only when every
child is simple; custom display follows its base. Shapeless recipes use item lists
when all ingredients are simple and otherwise test each concrete stack. Shaped
recipes always test the concrete stack. Recipe-book craftability, placement,
recipe property sets and network ingredients continue to use item lists.

## Component decoder support

Component ids are checked against the loaded registry, including removal keys.
Invalid payloads and unsupported built-in component patches reject the entire
recipe, with a warning naming the recipe and component. Native registrations
use their persistent NBT validation contract. Equality uses decoded values and
candidate matching does not allocate or convert values to JSON.

The shared decoder currently supports `damage`, `repair_cost`, `max_damage`,
`max_stack_size`, `rarity`, `enchantments`, `stored_enchantments`, `custom_name`,
`item_name`, and `block_entity_data` in the `minecraft` namespace.

The following registered 26.3 built-ins do not yet have a shared payload decoder
and are refused in component ingredient patches (all names use `minecraft:`).
This includes three registered transient types: `additional_trade_cost`,
`creative_slot_lock` and `map_post_processing`; no persistent codecs are invented
for them. The remaining 109 entries are persistent types lacking a shared decoder:


```text
additional_trade_cost
attack_animation
attack_range
attribute_modifiers
axolotl/variant
banner_patterns
base_color
bees
block_state
block_transformer
blocks_attacks
break_sound
brewing_fuel
bucket_entity_data
bundle_contents
can_break
can_place_on
cat/collar
cat/sound_variant
cat/variant
charged_projectiles
chicken/sound_variant
chicken/variant
compostable
consumable
container
container_loot
cooking_fuel
cow/sound_variant
cow/variant
creative_slot_lock
cushion/color
custom_data
custom_model_data
damage_resistant
damage_type
death_protection
debug_stick_state
dye
dyed_color
enchantable
enchantment_glint_override
entity_data
equippable
firework_explosion
fireworks
food
fox/variant
frog/variant
glider
horse/variant
instrument
intangible_projectile
interact_animation
item_model
jukebox_playable
kinetic_weapon
llama/variant
lock
lodestone_tracker
lore
map_decorations
map_id
map_post_processing
minimum_attack_charge
mob_visibility
mooshroom/variant
note_block_sound
ominous_bottle_amplifier
painting/variant
parrot/variant
piercing_weapon
pig/sound_variant
pig/variant
pot_decorations
potion_contents
potion_duration_scale
profile
provides_banner_patterns
provides_pottery_pattern
provides_trim_material
rabbit/variant
recipes
repairable
salmon/size
sheep/color
shulker/color
sign_text_back
sign_text_front
sulfur_cube_content
suspicious_stew_effects
tool
tooltip_display
tooltip_style
trim
tropical_fish/base_color
tropical_fish/pattern
tropical_fish/pattern_color
unbreakable
use_cooldown
use_effects
use_remainder
villager/variant
villager_food
waxed
weapon
wolf/collar
wolf/sound_variant
wolf/variant
writable_book_content
written_book_content
zombie_nautilus/variant
```
