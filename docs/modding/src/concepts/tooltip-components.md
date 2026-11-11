# Design: tooltip components in full

Provisional contract for #2927. This page proposes an API, not an implemented
feature. The maintainer's room specification is unavailable; the issue's five
requirements are authoritative here. Remaining bytecode and public-documentation
gaps below are shipping gates, not inferred NeoForge behavior.

## Current owners and baseline

References are to main `514ecf9a96`. This design builds on PR #2974
(`fix/bundle-tooltip-element`): `TooltipImage.stack` becomes
`source: TooltipImageSource { ItemPart(ItemStack), Bundle(ItemStack) }`.
The bundle enters the same ordered list at `components.isEmpty() ? 0 : 1`;
it is no longer appended outside layout. Generalise that source into a component
type and data, rather than adding another item-dispatched special case.

| Contract | Today's owner | Required change |
| --- | --- | --- |
| Item font, then Pre font, controls measurement | `crates/horizon-mod-vocab/src/hooks/render_tooltip_pre.rs:21`; `crates/horizon-client/src/game/blocks/stations/tooltip.rs:48`, `:137`, `:212` | Pre currently has no font field; wrapping and widths use the atlas font. Carry the selected font explicitly. |
| Exact component-type factory, arbitrary data | `crates/horizon-mod-vocab/src/hooks/tooltip_component.rs:16`, `tooltip_component_draw.rs:18`; `crates/horizon-mod-vocab/src/item/tooltip_element.rs:23` | Replace item-keyed sizing/drawing parts and stack-only data with registration-time typed factories. |
| Height, width, text drawing and image drawing | `crates/horizon-client/src/game/blocks/stations/tooltip.rs:134`, `:149`, `:244` | The two loops exist, but custom images only receive the image pass. Call both methods on each prepared custom component. |
| Only text wraps, once | `crates/horizon-client/src/game/blocks/stations/tooltip.rs:183`, `:218`, `:229`; `crates/horizon-client/src/game/blocks/stations.rs:12` | Text currently splits when max_width is nonnegative; invalid image sizes silently disappear. Preserve components, refuse invalid mod output explicitly, and settle the missing width rule. |
| Whole-tooltip cancellation | `crates/horizon-mod-vocab/src/hooks/render_tooltip_gather_components.rs:21`, `render_tooltip_pre.rs:21`, `render_tooltip_texture.rs:18`; `crates/horizon-client/src/game/blocks/stations/tooltip.rs:57`, `:209` | Gather empties the list; Pre skips the whole tooltip; neither component callback has cancellation. Texture stays noncancellable. |

`crates/horizon-mod-vocab/src/hooks/notes.rs:1744–1750` describes the old
stack-based contract and must change with implementation. The adapters are
`crates/horizon-client/src/game/native_hooks/base.rs:3396`, `:3406`, and
`native_hooks/render_tooltip_{gather_components,pre,texture}.rs:15`, `:9`, `:9`.

All item routes converge on `draw_stack_tooltip_with` (`tooltip.rs:82`), including
ordinary slots (`stations.rs:2156`, `:3143`), beacon (`game/blocks/beacon.rs:597`),
trades (`game/items/trading.rs:908`), creative (`game/interface/creative_screen.rs:298`),
chat hover (`game/interface/chat.rs:1805`), statistics
(`game/interface/stats_screen.rs:670`), native panels
(`game/modding/native_panel.rs:518`) and dialogs (`game/interface/dialogs.rs:776`).
Paths in this paragraph are relative to `crates/horizon-client/src/` unless qualified.
Text-only tooltips converge on `game/blocks/stations.rs:2867`; creative tab labels
also have `game/interface/creative_screen.rs:59`. Both must use the selected-font
Pre/measure/draw path. Recipe-book labels (`game/items/recipe_book.rs:2016`,
`game/items/recipe_book/mod_screen.rs:368`), potion labels
(`game/items/potions.rs:3605`), chat text (`game/interface/chat.rs:1778`),
recipe panels (`game/modding/native_panel_recipes.rs:663`), native panel labels
(`game/modding/native_panel.rs:511`), fluid labels
(`game/native_hooks/fluid_draw.rs:30`) and shared screen labels
(`game/native_hooks/shared.rs:3343`) use that text path. They have no item
Gather event. The bundle's selected-item label remains part of bundle drawing,
not a recursive tooltip event. All are client presentation routes for local,
LAN and remote play; dedicated servers register no renderer.

## Vanilla and NeoForge evidence

Use only vanilla 26.3 bytecode through these reproducible queries:

```sh
python3 tools/rea_vanilla.py class net.minecraft.client.gui.GuiGraphicsExtractor
python3 tools/rea_vanilla.py method net.minecraft.client.gui.GuiGraphicsExtractor tooltip
python3 tools/rea_vanilla.py --overload-index 1 method net.minecraft.client.gui.GuiGraphicsExtractor setTooltipForNextFrameInternal
python3 tools/rea_vanilla.py --overload-index 5 method net.minecraft.client.gui.GuiGraphicsExtractor setTooltipForNextFrame
python3 tools/rea_vanilla.py class net.minecraft.client.gui.screens.inventory.tooltip.ClientTooltipComponent
python3 tools/rea_vanilla.py --overload-index 1 method net.minecraft.client.gui.screens.inventory.tooltip.ClientTooltipComponent create
python3 tools/rea_vanilla.py class net.minecraft.client.gui.screens.inventory.tooltip.ClientTextTooltip
python3 tools/rea_vanilla.py class net.minecraft.world.inventory.tooltip.TooltipComponent
python3 tools/rea_vanilla.py class net.minecraft.world.inventory.tooltip.BundleTooltip
python3 tools/rea_vanilla.py method net.minecraft.client.gui.screens.inventory.tooltip.ClientTextTooltip getHeight
python3 tools/rea_vanilla.py method net.minecraft.client.gui.screens.inventory.tooltip.ClientTextTooltip getWidth
python3 tools/rea_vanilla.py method net.minecraft.client.gui.screens.inventory.tooltip.TooltipRenderUtil extractTooltipBackground
python3 tools/rea_vanilla.py callers net.minecraft.client.gui.GuiGraphicsExtractor tooltip
```

`setTooltipForNextFrame` has twelve overloads (indices 0–11). Verified overload 0
converts a Component to visual-order text and delegates; overload 1 supplies
`this.minecraft.font` and `DefaultTooltipPositioner.INSTANCE`. Other signatures
accept an explicit Font, item stack, formatted text, optional TooltipComponent,
positioner, style and a boolean. Verified overload 2 forwards its supplied Font, item lines, image and style
with extra spacing enabled; overload 3 forwards with null style. Overload 5
converts visual-order text to client components, inserts optional image data at
index zero or one, and forwards the supplied Font unchanged.
`setTooltipForNextFrameInternal` overload 1 captures that Font in `deferredTooltip`
and calls `tooltip` with it. The verified `tooltip` body measures maximum
`getWidth(font)` and sums `getHeight(font)`, starting height at -2 except when
extra spacing is enabled with multiple components. It positions the content box,
extracts its background, runs every text extraction, then every image extraction.
Vanilla callers include the deferred lambda and
`ClientBundleTooltip.extractSelectedItemTooltip` (a direct tooltip call). Both
loops advance by component height plus 2 after the first element when extra
spacing is enabled. Images receive the whole content box. This renderer performs
no wrapping or resizing. All twelve overload bodies were inspected. Overload 4 defaults extra spacing
to false; 6 converts formatted sequences and inserts optional data at index zero
or one; 7–9 delegate through the style overloads; 10 and 11 convert formatted
sequences and use the internal deferral with default or supplied positioner.
The shorter internal overload defaults extra spacing to false. None of these
bodies splits text. Thus supplied lists reach the renderer without a width cap
or wrapping in this vanilla path. The room's precise additional width/wrap rule
is still unverified; do not invent a screen-width clamp.

`ClientTooltipComponent` exposes `getHeight(Font)`, `getWidth(Font)`,
`extractText(GuiGraphicsExtractor, Font, x, y)` and
`extractImage(Font, x, y, width, height, GuiGraphicsExtractor)`: the 26.3 names
for the issue's renderText/renderImage responsibilities. `ClientTextTooltip`
implements height, width and text extraction. `TooltipComponent` is a data marker
with no methods. Verified `create(TooltipComponent)` maps `BundleTooltip.contents()`
to `ClientBundleTooltip`, maps active-player data to its client component, and
throws `IllegalArgumentException` for unknown data. This supports a build-time
error rather than silently skipping unsupported data. `BundleTooltip` stores `BundleContents`. `ClientTextTooltip.getWidth` calls
`font.width(text)`. `getHeight` returns 10 and `extractText` draws with the supplied font.
`TooltipRenderUtil.extractTooltipBackground` positions both sprites at
(x-12,y-12), with dimensions (w+24,h+24): its arithmetic is 3 pixels of
content padding plus 9 pixels of sprite extension per side.

Public NeoForge 26.3 javadoc for `RenderTooltipEvent.GatherComponents` confirms
mutable elements and maximum width before wrapping, client-only dispatch, and
cancellation suppressing both Pre and Texture. `RenderTooltipEvent.Pre` confirms
mutable font and position and cancellation suppressing Texture. Its screen-width
documentation says lines wrap to screen width. `RenderTooltipEvent.Texture`
confirms mutable background/frame texture and no cancellation.
`RegisterClientTooltipComponentFactoriesEvent.register(Class<T>, Function<...>)`
is publicly indexed as a factory registration for component data types. Its full
page was unavailable. Exact-class matching, duplicate replacement, registration
bus timing, missing-factory behavior and the item-font-before-Pre sequence are
**issue requirements, not independently verified NeoForge implementation facts**.
No NeoForge source or patches were consulted. The
[public GUI tooltip documentation](https://docs.neoforged.net/docs/gui/tooltips/)
was inaccessible; the room's width rule cannot be reconstructed from it.

## Proposed public contract

Provisional API spelling: a client-load `register_client_tooltip_component_factories`
registrar with `register<T: Wire>(TooltipComponentType<T>, factory)`. The exact
component key identifies one data schema; no subtype or item fallback. Registering
the same key again replaces its factory in dependency dispatch order. Freeze the
map after client registration and refuse replacement with a different Wire schema
for the same key. Resolve string keys to slots there, never during
drawing. Reuse registry/lifecycle ownership and scoped callback resources; no
second generation counter. Reserved built-in types cannot be impersonated by
mod keys. An item supplies component data, not a renderer keyed by its item id.
Gather may insert the same type with different payloads anywhere in the list.

Replace `TooltipImageSource::ItemPart` with a registered component source carrying
an API-owned type key and opaque Wire-encoded payload. Keep bundle as the built-in
source from #2974 until it can use the same internal dispatch without changing its
vanilla result. A typed registrar hides byte encoding from mod authors; arbitrary
user data means any supported Wire schema, not native pointers, Java objects or
unbounded allocation. New keys, payload envelopes, selected-font references,
measurement records and error enums all have API-owned Wire forms and `///` docs.
Native and wasm use the same schema identity, registrar, validation and callbacks.

A factory creates a prepared component with `width(font)`, `height(font)`,
`draw_text` and `draw_image`. Callbacks receive `&dyn ClientHost`; only drawing
receives DrawContext. Store callback resources through existing runtime ownership,
not serialized closures. Pass the component's data, selected font, origin and whole
content-box size to both passes. Finish all text callbacks in element order, then
all image callbacks in that same order with the same offsets. There is no
per-component cancellation. Remove the two old item parts and update all callers,
catalog entries, generated adapters and docs together; no compatibility aliases.

Missing factory produces proposed `MissingTooltipComponentFactory(type_key)`
while preparing the tooltip, before its background or any component draws. It
fails that tooltip with a diagnostic, never silently removes one element.
Schema mismatch or malformed payload similarly refuses preparation. Registration
must return proposed `UnsupportedCapability(tooltip_components)` if either target
cannot supply the complete contract; never accept a callback that cannot run.
These are proposed error names, not existing symbols.

Select the item tooltip font (or the ordinary supplied/default font), then let Pre
replace a Wire font reference resolved by ClientHost. Pass that exact resolved font
to final width/height queries and both drawing passes; do not read the atlas again.
Gather mutates raw elements. Text wrapping happens exactly once and components
are neither wrapped nor resized. The ordering between wrapping and Pre is an
explicit gate below: do not implement a second wrap when Pre changes fonts.
Gather cancellation yields an empty list and no Pre/Texture; Pre cancellation
skips the entire remaining pipeline. Existing drawing a listener explicitly emits
inside Pre has its existing semantics, separate from component cancellation.

## Bounds, failure and unused cost

Use existing codec bounds only: `crates/horizon-mod-vocab/src/wire/mod.rs:4`
specifies u32 collection lengths, checked against available encoded bytes;
`:92` owns `MAX_DEPTH = 32`; `:477` validates lengths against the remaining input.
Text conversion separately owns `MAX_TEXT_DEPTH = 512` at
`crates/horizon-mod-vocab/src/text/mod.rs:476`. Do not turn those into a new tooltip
count, pixel, payload-byte or screen-width cap. Payloads retain the owning Wire
transport's configured budget. Native inputs receive the same structural checks.

Validate mod measurements before arithmetic: reject nonfinite, negative or
unrepresentable dimensions and checked-sum overflow. Zero-sized components need
an explicit decision below; do not inherit today's strictly-positive filter as
vanilla law. No clamp or resize to screen width. A trap/panic or invalid component
answer aborts preparation of the whole tooltip with existing runtime fault
isolation and diagnostics. Stage component draw commands so a failed draw cannot
leave half a component on screen; discard that tooltip's staged commands on failure.
A bad listener answer must not poison the next ordinary tooltip. No game-state
mutation is required; any allowed listener transaction follows existing root-commit
and rollback rules.

With no subscribers or custom data, take O(1) presence checks and run exactly the
vanilla path: no API element conversion, payload encoding, registry scan, factory
allocation or new validation/clamp/minimum. Built-in constants and default font,
wrapping, positioning and bundle behavior remain vanilla. Today's unconditional
API list conversions in `tooltip.rs:120` and `:190` are costs to remove, not the
promised baseline. With custom data use indexed exact-key dispatch and prepare
each component once; no per-frame string hashing or formatting.

## Acceptance matrix

These are independent hand-computed targets, not tests that call production layout
to generate their expectations. The issue requires a 2-pixel gap after the first
component. The following uses verified vanilla constants: 10-pixel text height,
single-component height correction of -2, extra spacing enabled for these item
fixtures, and a content origin (40, 30). The background/frame sprite rectangle
starts at (28,18) and has width W+24 and height H+24. Its inner content padding
is 3 pixels; the sprites additionally extend 9 pixels per side.

| Fixture | Expected content W × H; origins | Observable result |
| --- | --- | --- |
| One text, measured width 24 | 24 × 8; (40,30) | Text once; sprite box (28,18,48,32) |
| Two text rows, widths 24 and 18 | 24 × 20; (40,30), (40,42) | 10+10 content height; first advance 10+2 |
| Text 24×10, component 30×16, text 18×10 | 30 × 36; y=30,42,58 | All text callbacks, including custom text, precede every image callback; box passed to image is 30×36 |
| Component 30×16 then text 24×10 | 30 × 26; y=30,48 | Gap belongs after first element even when it is custom |
| Same type, payloads yielding 30×16 and 12×8 | 30 × 24; y=30,48 | One registered factory, two distinct prepared values; no item identity lookup |
| Pre selects font B: row width 36 instead of A's 24; component width B=42 | 42 × 26 for row 10 + component 16; y=30,42 | Measure and draw both use B; fixed-atlas width fails |
| Oversized custom component 400×16 beside text 24×10 | 400 × 26; y=30,42 | Component retains width 400 even on a 320-wide screen |
| Gather cancelled | Empty | No factory, Pre, Texture or component draw |
| Pre cancelled | No tooltip output | No Texture or component drawing |
| Unknown custom type or malformed Wire payload | Preparation error | No silent skip; next built-in tooltip matches vanilla |
| Duplicate registration A then B | B's dimensions and drawing | Exact key replacement, on native and wasm |
| Unsupported runtime capability | Registration refusal | No inert registered factory |

Add bytecode-confirmed wrapping fixtures for unset width, positive max width,
screen-edge fallback and font replacement before implementing that stage; expected
line boundaries must be written by hand using a fixed font fixture. Test images
between wrapped text, an image-only tooltip, zero-size decisions, invalid floats,
overflow, panic/trap recovery and no-listener equivalence on both targets.

## Stages

1. Resolve the evidence gates below and reconcile #2974's ordered source shape.
   Resolve the room width rule and its ordering relative to Pre. Do not land guessed layout behavior.
2. Add the registrar, typed data envelope, font reference and prepared component
   interface with Wire and documented refusals on native and wasm together.
3. Replace old parts, route item and text-only consumers through shared preparation,
   implement two complete passes and cancellation, retaining the exact unused path.
   Use a sibling module rather than growing oversized `stations.rs` or `notes.rs`.
4. Add independent layout/dispatch/loopback tests for the matrix; update generated
   boundary documentation and a user-facing book example. This note changes no Rust.

## Open questions

- What is the exact width/wrap formula, and does Pre change only final measurement
  or also the font used for the single wrapping step? Recommendation: obtain the
  room specification or finish vanilla/public evidence; preserve one wrap and pass
  Pre's font to measurement regardless. Do not silently reorder NeoForge moments.
- Which public font reference and item-font selection service should be exposed?
  Recommendation: a typed client registry reference shared by measurement and
  DrawContext, resolved once; avoid an opaque native Font pointer or a new hook at
  the same moment as an existing item customization point.
- Are zero width/height legal, and are there additional component-specific size
  constraints? Recommendation: verify bytecode and
  allow representable zero if vanilla does; never impose today's positive filter
  on unchanged built-in output.
- Can the runtime supply scoped prepared callback resources and atomic draw staging
  on both targets? Recommendation: use existing lifecycle ownership; make complete
  support an admission gate rather than shipping native-only behavior.
- Can public NeoForge documentation confirm replacement/exact lookup/error timing?
  Recommendation: retain the issue's explicit contract, label independent parity
  unverified until public evidence is available; never inspect sources or patches.
