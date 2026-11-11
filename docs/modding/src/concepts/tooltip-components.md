# Design: tooltip components in full

Provisional contract for #2927. This page proposes an API, not an implemented
feature. The issue and maintainer's contract supplied in the #3129 review govern this
proposal. Public NeoForge evidence is distinguished from that supplied contract;
no NeoForge implementation source or patches are needed to adopt the contract.

## Current owners and baseline

Every file:line below was re-verified against fetched fork main `4ee6ab60696fda68c1d99b05ffd972edec0b9a74`.
`TooltipImageSource` is not on that main. This design depends on PR #2974
landing first; the #2927 build must come after it. PR #2974
(`fix/bundle-tooltip-element`): `TooltipImage.stack` becomes
`source: TooltipImageSource { ItemPart(ItemStack), Bundle(ItemStack) }`.
The bundle enters the same ordered list at `components.isEmpty() ? 0 : 1`;
it is no longer appended outside layout. Generalise that source into a component
type and data, rather than adding another item-dispatched special case.

| Contract | Today's owner | Required change |
| --- | --- | --- |
| Item font, then Pre font, controls measurement | `crates/horizon-mod-vocab/src/hooks/render_tooltip_pre.rs:21`; `crates/horizon-client/src/game/blocks/stations/tooltip.rs:48`, `:137`, `:212` | Pre currently has no font field; wrapping and widths use the atlas font. Carry the selected font explicitly. |
| Exact component-type factory, arbitrary data | `crates/horizon-mod-vocab/src/hooks/tooltip_component.rs:16`, `tooltip_component_draw.rs:18`; `crates/horizon-mod-vocab/src/item/tooltip_element.rs:24` | Replace item-keyed sizing/drawing parts and stack-only data with registration-time typed factories. |
| Height, width, text drawing and image drawing | `crates/horizon-client/src/game/blocks/stations/tooltip.rs:134`, `:149`, `:244` | The two loops exist, but custom images only receive the image pass. Call both methods on each prepared custom component. |
| Only text wraps, once | `crates/horizon-client/src/game/blocks/stations/tooltip.rs:183`, `:218`, `:229`; `crates/horizon-client/src/game/blocks/stations.rs:12` | Text currently splits when max_width is nonnegative; invalid image sizes silently disappear. Preserve components, refuse invalid mod output explicitly, and wrap before Pre under the supplied contract. |
| Whole-tooltip cancellation | `crates/horizon-mod-vocab/src/hooks/render_tooltip_gather_components.rs:21`, `render_tooltip_pre.rs:21`, `render_tooltip_texture.rs:18`; `crates/horizon-client/src/game/blocks/stations/tooltip.rs:57`, `:209` | Gather empties the list; Pre skips the whole tooltip; neither component callback has cancellation. Texture stays noncancellable. |

`crates/horizon-mod-vocab/src/hooks/notes.rs:1744–1750` describes the old
stack-based contract and must change with implementation. The adapters are
`crates/horizon-client/src/game/native_hooks/base.rs:3377`, `:3387`, and
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
(`game/native_hooks/shared.rs:3267`) use that text path. They have no item
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
python3 tools/rea_vanilla.py --overload-index 0 method net.minecraft.client.gui.screens.inventory.tooltip.DefaultTooltipPositioner positionTooltip
python3 tools/rea_vanilla.py --overload-index 1 method net.minecraft.client.gui.screens.inventory.tooltip.DefaultTooltipPositioner positionTooltip
python3 tools/rea_vanilla.py method net.minecraft.client.gui.GuiGraphicsExtractor guiWidth
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
or wrapping in this vanilla path. The additional wrapping stage below belongs
to the supplied maintainer contract, not these vanilla method bodies.

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

Verified `DefaultTooltipPositioner.positionTooltip` starts at (mouseX+12,
mouseY-12). If startX+width exceeds screenWidth, it changes X to
`max(startX-24-width, 4)`; if Y+height+3 exceeds screenHeight, it changes Y to
`screenHeight-height-3`. There is no second right-edge clamp or component resize.
`GuiGraphicsExtractor.guiWidth` returns the window's GUI-scaled width. These are
vanilla **positioning** rules, not a text-wrapping formula.

Public NeoForge 26.3 javadoc for `RenderTooltipEvent.GatherComponents` confirms
mutable elements and maximum width before wrapping, client-only dispatch, and
cancellation suppressing both Pre and Texture. `RenderTooltipEvent.Pre` confirms
mutable font and position and cancellation suppressing Texture. Its screen-width
documentation says lines wrap to screen width. `RenderTooltipEvent.Texture`
confirms mutable background/frame texture and no cancellation.
`RegisterClientTooltipComponentFactoriesEvent.register(Class<T>, Function<...>)`
is publicly indexed as a factory registration for component data types. Its full
page was unavailable. The supplied maintainer contract now settles the registration
and font/wrapping decisions: the registrar fires once on the client before the
first resource load; later registration silently replaces earlier registration;
built-in types match first; unknown types fail during tooltip construction.
Wrapping occurs once before Pre with the item's tooltip font. It starts with
the widest text element (components contribute zero), applies screen fitting,
then a positive maximum-width cap. The screen-fit arithmetic remains an open
gate below; the supplied review establishes its order, not its formula. Pre's
font controls final measurement and drawing.
These are accepted contract facts from the #3129 review, not a claim to have
independently verified NeoForge internals. No NeoForge source or patches were
consulted. The [public GUI tooltip documentation](https://docs.neoforged.net/docs/gui/tooltips/)
was inaccessible; this does not leave the supplied contract open.

## Proposed public contract

Provisional API spelling: a client-load `register_client_tooltip_component_factories`
registrar, fired once on the client before the first resource load, with
`register<T: Wire>(TooltipComponentType<T>, factory)`. The exact
component key selects the current factory; each payload must decode under that
factory's Wire schema, with no subtype or item fallback. Registering
the same key again silently replaces its factory in dependency dispatch order.
Choose replacement even if the new factory expects a different Wire schema;
check schema identity when decoding each payload, not when replacing registration.
Built-in types match first, then use an exact custom-key lookup. Freeze the map
after client registration. Resolve string keys to slots there, never during
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
Gather mutates raw elements. Before Pre, wrap text exactly once using the item
tooltip font: start from the widest text element, counting components as zero;
apply the screen-fit rule (arithmetic pending maintainer confirmation), then cap
to max_width only when it is positive.
Components are neither wrapped nor resized. Pre may replace the font for final
measurement and drawing, but never triggers a second wrap or changes those line
boundaries. This ordering is the supplied maintainer contract.
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
vanilla law. Component dimensions are never clamped or resized to screen width;
vanilla's positioner still flips and clamps the box position. A trap/panic or
invalid component answer aborts preparation of the whole tooltip with existing runtime fault
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
fixtures, and a content origin (40, 30) assumed to be the positioner's result
unless a row supplies its own positioning inputs. The background/frame sprite
rectangle starts at (28,18) and has width W+24 and height H+24. Its inner content padding
is 3 pixels; the sprites additionally extend 9 pixels per side.

| Fixture | Expected content W × H; origins | Observable result |
| --- | --- | --- |
| One text, measured width 24 | 24 × 8; (40,30) | Text once; sprite box (28,18,48,32) |
| Two text rows, widths 24 and 18 | 24 × 20; (40,30), (40,42) | 10+10 content height; first advance 10+2 |
| Text 24×10, component 30×16, text 18×10 | 30 × 36; y=30,42,58 | All text callbacks, including custom text, precede every image callback; box passed to image is 30×36 |
| Component 30×16 then text 24×10 | 30 × 26; y=30,48 | Gap belongs after first element even when it is custom |
| Same type, payloads yielding 30×16 and 12×8 | 30 × 24; y=30,48 | One registered factory, two distinct prepared values; no item identity lookup |
| Pre selects font B: row width 36 instead of A's 24; component width B=42 | 42 × 26 for row 10 + component 16; y=30,42 | Measure and draw both use B; fixed-atlas width fails |
| Oversized custom component 400×16 after text 24×10; cursor (28,42), screen 320×240 | 400 × 26; origins (4,30), (4,42) | Start X=40; max(40−24−400,4)=4. Only component size stays 400; the box position changes and still overflows the screen. |
| Positioner right-edge flip; cursor (300,80), screen 320×240, content 60×20 | Content origin (228,68) | Start (312,68); 312+60>320; X=max(312−24−60,4)=228. Y+20+3=91 fits. |
| Gather cancelled | Empty | No factory, Pre, Texture or component draw |
| Pre cancelled | No tooltip output | No Texture or component drawing |
| Unknown custom type or malformed Wire payload | Preparation error | No silent skip; next built-in tooltip matches vanilla |
| Duplicate registration A then B | B's dimensions and drawing | Exact key replacement, on native and wasm |
| Unsupported runtime capability | Registration refusal | No inert registered factory |

### Wrapping fixtures

The supplied contract's order is: **wrap width starts from the widest text
element, components count 0, then the screen-fit rule, then the positive
max-width cap**. Screen-fit arithmetic is pending, so only fixtures whose text
fits on either side have final expected boundaries here. Use a fixed monospace
font of 6 pixels per character, text height 10, and a deterministic hard break
inside an unspaced word when it exceeds the width. This is a test font fixture,
not an assertion about resource-pack glyph widths. These boundaries are chosen
before Pre; changing the Pre font never splits them again.

| Fixture | Hand-written expected lines | Size and derivation |
| --- | --- | --- |
| No max width (`-1`), screen 320×240, mouse (100,80), text `ABCD`, custom component 60×16 | [`ABCD`] | Text width 4×6=24; component contributes 0 to wrap width. Both sides have room, so no screen-fit ambiguity. Final content is 60×26 with text then component. |
| Positive max width 24, same screen/mouse, text `ABCDEFGH` | [`ABCD`, `EFGH`] | Initial text width 8×6=48 fits either side. Cap to 24 permits 4 characters per row. Final content 24×20; relative Y offsets 0 and 12. |
| Screen-fit case: screen width 100, mouse X=40, no max width, text `ABCDEFGHIJ` | **Pending confirmed screen-fit formula** | Initial width 10×6=60; components still contribute 0. Do not freeze line boundaries or derive them from the implementation until maintainers settle the arithmetic. |

Test images between wrapped text, an image-only tooltip, zero-size decisions,
invalid floats, overflow, panic/trap recovery and no-listener equivalence on both
targets. The pending row is a build gate, not permission to guess its expected value.

## Stages

1. Land #2974 and obtain maintainer confirmation of screen-fit arithmetic first,
   then implement #2927 on that ordered-source shape.
2. Add the once-only client registrar before first resource load, Wire envelope,
   font reference and prepared component interface on native and wasm together.
3. Remove both old item parts and update every caller, catalog entry, generated
   adapter and documentation together. Share preparation across item and text-only
   routes, with the complete two-pass contract and the no-listener fast path.
4. Add independent layout/dispatch/loopback tests, regenerate boundary docs and
   add a user-facing book example. This note changes no Rust.

## Build issue

The #2927 implementation starts only after #2974 lands and the screen-fit formula
is confirmed, and carries stages 2–4 as one complete native/wasm contract: registrar, Wire data envelope, font reference,
prepared component, removal of both old parts with every caller and the catalog,
shared item/text-only preparation, exact vanilla no-listener fast path and a book
example. Keep new logic in sibling modules rather than growing oversized owners.

Start with failing tests: every acceptance-matrix row must have its own test with
the literal hand-written sizes, origins, ordering or refusal stated here. Add all
wrap fixtures with literal expected line boundaries (fill in the pending
screen-fit row after confirmation), the positioner-flip fixture,
and a no-listener equivalence test on **both native and wasm**. No expected result
may be generated by calling the implementation under test. Tests must distinguish
the item's wrapping font from Pre's measurement/drawing font, silent replacement
from decode-time schema refusal, and unchanged component size from changed box
position. Trap/invalid-output recovery must be followed by a built-in tooltip.

## Open questions

- What is the screen-fit arithmetic? The review gives the order but no formula,
  and the verified vanilla path does not wrap. Candidate (a) caps at
  `screenWidth − (mouseX + 12) − 4`; candidate (b) tries the right side, then the
  left, and wraps only if neither fits. Recommendation: maintainers must confirm
  the formula before the build; choose neither by inference from the positioner.
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
- Wrapping assumes a box at mouse X+12; a wide custom component can move the
  box later during positioning. This is expected vanilla positioning behavior,
  not a wrapping bug. Recommendation: retain the one text wrap before Pre and the
  later positioner result; do not rewrap or resize the component after it moves.
