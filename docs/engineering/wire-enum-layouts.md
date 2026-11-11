# Fieldless Wire enum layout pins

`tools/fixtures/wire_enum_layouts.json` is a generated semantic baseline, not a
hand-maintained type registry. The initial capture used native default-feature
`horizon-mod-vocab` at base `5c6c41f19f64c26231e0e49be505c5e2b1439be9`, normal
opt0, before the fieldless-enum const fold. All 151 discovered enums were called
through their actual compiled `Wire::layout` implementations. Generated/private
`HookId` is included. No cloned type supplied these expected hashes.

The generated ordinary Rust test calls each actual type, with no unsafe code.
The generator independently discovers unit-variant enums and Wire impls from
compiler expansion and requires exact identity coverage against the manifest.
Ambiguous names and unsupported generic/qualified/where-clause impl headers for
local unit enums fail rather than silently dropping a type. The 151 pins cover
locally defined fieldless Wire derives, including generated/private enums. They
do not cover external types with manual codecs (such as horizon-core's Direction
or horizon-mod-runtime's attachment types), which this derive change cannot affect.
The checked configuration is native/default features, not inactive target or
feature branches.

Two generated fixture paths were corrected using rustc E0603 re-export
suggestions: EditKey and ProvidedBy. This is compiler-guided fixture data, not
automatic alias resolution in the generator. Each manifest entry retains its
canonical symbol identity; those two also retain the correction provenance.

Regenerate or check without running the game:

```sh
mkdir -p target/wire-goldens
RUSTC_BOOTSTRAP=1 cargo rustc --locked -p horizon-mod-vocab --lib -- \
  -Zunpretty=expanded > target/wire-goldens/expanded.rs
python3 tools/generate_wire_enum_goldens.py --expanded target/wire-goldens/expanded.rs
# CI uses --check instead of writing the generated test.
cargo test --locked -p horizon-mod-vocab --lib existing_fieldless_enum_layouts_match_baseline
```

Bootstrap here enables compiler expansion only; it is not a production build
flag. Output stays in checkout `target/`. CI runs both the exact discovery-set /
generated-file check and the ordinary Rust golden test in the existing wasm-mods
job; its path filters include the derive crate and these generator inputs.

For intentional enum changes, capture the **before-change** actual layout values
first; record the source revision/configuration and the complete discovery set.
New enum hashes must be checked against independent expected encoding/layout
semantics, not blindly blessed from the changed derive. Update the manifest by
generated capture, then regenerate the Rust file with the command above and
review the changed hashes and wire bytes. Never hand-add names to the generated
Rust registry. Reordering/removing existing variants is an interface change, not
a reason to casually regenerate expected values. Separate small independent
golden-byte/hash tests in `wire::tests` guard against matching implementation
errors in a mechanically captured baseline.
