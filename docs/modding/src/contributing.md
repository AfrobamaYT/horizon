# Where each doc lives

The book follows [Diátaxis](https://diataxis.fr): a page is one of four kinds, and does only that job.

| Kind | Job | Lives in |
|---|---|---|
| Tutorial | Take a newcomer to a working result | Start |
| Explanation | Say how the pieces fit, with no steps | Concepts |
| How-to | One task, steps only | How-to, Injections |
| Reference | Facts, complete and dry | Reference, and the rustdoc of `horizon-mod-api` |

- **A hook, event or `Host` method:** its `///` lines in the declaration under `crates/horizon-mod-api/src/hooks/`.
  The generator writes it as `///` on the generated item; `cargo doc -p horizon-mod-api` shows it. A hook without
  a `doc` fails the build.
- **A hook whose game target has many parameters:** a `native_*` target never carries
  `#[allow(clippy::too_many_arguments)]`. Declare the parameters on the hook instead
  (`fn update_shape(dimension: LevelDimension, ...);`, as written in `hooks/*.rs`) and the target takes them as one generated
  `mod_api::__private::<Name>Call` (`native_update_shape(&self, ticks, call: UpdateShapeCall<'_>)`), which it hands to
  `dispatch_<name>(host, call)`. The generator refuses a target with the allow. Other targets stay flat on purpose:
  the carrier is host-internal plain data, never on wasm, and no modder sees it.
- **A guide page:** `docs/modding/src/`. Code in a page is `{{#include}}`d from `docs/modding/examples`, which
  compiles, so a change to the API that breaks an example fails the build.
- **A mod API change** updates its fragment `doc` and any page it affects in the same PR.
- `src/llms.txt` is the index for agents; keep it in step with `SUMMARY.md`.

Build the book with `mdbook build docs/modding` and the reference with `cargo doc -p horizon-mod-api --no-deps`.
