<!-- chapter: Named hook points -->
<!-- separator: 1 -->

### Named hook points

Implement catalog hooks as methods of `mod_api::Hooks`. An injection does not select a hook by
`hook` or `hook_version`: the manifest schema accepts `[[inject]]` with a target in `at`,
a crate-root listener in `with`, and the pinned function hash. See [Injections](../../injections.md).

Raw frame functions are native escape hatches, not methods on `Hooks`. For example, an
internals injection at `horizon_mod_render::render::world_drawn` names its listener with
`with`; use the current catalog hash and `kind = "head"`. The launcher supplies the
checked raw listener signature, and the ordinary registry owns its priority and lifetime.
