# API PR 2d: per-kind registration and capability traits

Base: `a790014309e9ea83930bd03d3f4b7257ad3eaef1` (API PR 2c).

The scripted move splits Registrar's declarations and PartRegistrar's implementations
into kind-local traits, alongside the existing kind-local helpers. CapabilityProviders
and InstalledProviders are split the same way. Existing call signatures and method
docs stay unchanged. The generic capability presence query belongs to capabilities.rs.

The Parts folder generator discovers the per-kind traits and generates the two union
supertraits in OUT_DIR. The API owns their documentation templates. The fixture
aggregator continues to discover exported fixtures from each kind's tests module;
wasm Guest implementations follow the same discovered supertraits as native ones.
No dispatch or registration behaviour changes, and no new shared method list is added.

Proofs: a moved-lines check with declared wrapper/import changes, host manifest and
WIT gate against the base above, focused Parts and generator tests, boundary tests,
and workspace check including tests and examples. New-kind coverage must compile an
added kind and exercise it through the union without changing existing shared files.
