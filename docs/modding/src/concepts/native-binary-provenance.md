# Design: native binary provenance

Provisional answer to [#804](https://github.com/arismoko/horizon/issues/804), shared with the producer in [#802](https://github.com/arismoko/horizon/issues/802). This proposes a contract; no verifier or workflow ships with this note. There is no NeoForge counterpart. Native libraries execute with the player's OS privileges, including library initialization before mod registration. Consent and provenance are both required; neither makes code safe.

## What exists

The [registry index](../reference/registry-index.md) already records binaries by exact build and target (#803). Verified against this checkout: `crates/horizon-mod-package/src/index.rs:191` owns `NativeBinary`; `:205` owns exact lookup; `:334` bounds build strings; `:350` checks binary file metadata; `:357` requires a decodable signature. `:511` owns the reusable minisign file verifier. Parsing a signature is not authenticating registry CI. Project author keys authenticate packages, not native CI provenance.

`launcher/src/native_mods.rs:2017` begins the registry lookup before source preparation; `:2036` only logs the selected URL; `:2046` proceeds to source preparation. This path does not install a verified registry binary. These are integration owners, not evidence that the proposed trust checks exist. Existing source builds execute Cargo build scripts and macros: automatically falling back to them after verification failure would bypass this boundary.

## Proposed shared envelope

A binary must prove that a trusted registry builder attested to these exact bytes, built from one named public source commit, for this exact Horizon build and target. It must bind the mod identity and build inputs too. A signature proves an attestation by a key holder, not that the source is benign or that CI actually followed its recipe.

Add one required provenance object to each native binary record. Use a detached minisign signature over a bounded UTF-8 payload file. Sign the exact payload bytes, including a domain prefix `horizon-native-provenance/1\n`; never reconstruct signed bytes from parsed JSON. After signature verification, parse the remaining JSON with duplicate and unknown keys refused. One representation is shared by #802 and #804, with no unsigned acceptance path.

Required payload fields:

| Field | Proposed meaning and bound |
| --- | --- |
| `schema`, `purpose` | Integer `1`, string `horizon-native-binary`; any other value refused. |
| `registry`, `key_id` | Bundled registry identity and key identifier, each at most 64 ASCII bytes; key ID selects only a bundled authorized key. |
| `mod_id`, `version` | Exact selected package identity; ID follows existing 64-byte rule, version at most 128 ASCII bytes. |
| `source_repository`, `source_commit` | Canonical public HTTPS repository URL, at most 2048 bytes, no credentials/query/fragment; full lowercase 40-hex Git commit, never a branch or tag. Other object formats require a new schema. |
| `package_sha256` | 64 lowercase hex digits identifying the reviewed source `.hmod`, independent of project signatures. |
| `build`, `target` | Exact #803 build string (1–128 bytes, no ASCII controls) and supported canonical Rust target triple (at most 128 bytes). |
| `toolchain`, `recipe_sha256`, `inputs_sha256` | Exact Rust release and compiler commit, at most 256 bytes; SHA-256 of the registry workflow recipe and locked dependency/input manifest. The manifest must include Cargo.lock, features, profile, game archive hash and relevant native toolchain versions. No floating dependency inputs. |
| `artifact_name`, `artifact_size`, `artifact_sha256` | Plain basename at most 255 bytes; integer 1–67108864; 64 lowercase hex digits. Bind the complete distributed file, not an extracted subset. |
| `builder`, `run_id`, `issued_at` | Authorized workflow identity (256 bytes), immutable run identifier (128 bytes), UTC timestamp. These support audit; a timestamp alone does not establish freshness. |

Envelope payload limit: 16 KiB; signature limit: 4 KiB; no embedded key or certificate can add trust. Keep the existing file signature as a registry-key signature over artifact bytes and require both signatures under the same authorized registry key. Neither `Project.key` nor a key supplied by the index is eligible. URLs are transport hints, not authority: derive the registry-owned release URL from validated identity and artifact name. Refuse project release binaries and redirects outside the configured registry release origin/path, including redirects to private or local network addresses.

## Verification before execution

One shared verifier must gate GUI, attached CLI, client, dedicated server, export and cache reuse. It returns an immutable verified identity and staged artifact, never just a boolean associated with a mutable path.

1. Read bounded index bytes (proposed 16 MiB), with at most 4096 projects, 256 versions per project and 64 binary records per version; enforce aggregate limits before allocation. Refuse duplicate IDs and build/target pairs, invalid integers, strings and URLs. Treat descriptions and errors as inert escaped text. Do not fetch a source URL to validate provenance.
2. Select the locally trusted game build ID and host target, then use exact lookup. `build` is the opaque exact identifier used by #803, not a game version, revision prefix or nearest release. Compare signed build, record build and installed game's trusted build identity byte for byte; target, mod/version and source-package hash must also match. A locally modified or unidentified game has no eligible registry binary.
3. Authenticate current trust/revocation metadata as described below. Bound envelope input, select a bundled authorized key, verify its detached payload signature, then parse and validate every payload field. Check source repository against reviewed package identity, not the index's display text alone.
4. Download only the registry-owned artifact into a new private staging file, enforcing the declared size while streaming. Check exact size, SHA-256, artifact signature and all envelope/record equalities. Refuse archives containing traversal, symlinks, duplicate paths or executable dependencies outside the verified artifact set; cap extracted total at 256 MiB and entries at 1024. No `dlopen`, executable probe, Cargo invocation or library inspection that executes code is permitted here.
5. Show the verified repository and full commit, mod/version, build, target and unrestricted native-code warning. Require explicit per-profile consent bound to payload hash and artifact hash. CLI without matching recorded consent refuses with an actionable diagnostic. Changed identity or bytes require new consent. Cancellation deletes staging and creates no consent or usable cache entry.
6. Atomically admit into a private content-addressed cache after verification and consent. Recheck hashes and current revocation before each launch/export; protect against symlink/path replacement and verify the same immutable file handed to the loader. Every separately loaded native dependency needs the same gate. Export retains signed provenance and must be reverified by the receiving launcher. Direct loading outside the launcher needs an equivalent loader gate; this note cannot certify that census.

Missing pair is a typed absence, not permission to execute another binary. Missing/invalid provenance, wrong key, expired trust metadata or any mismatch refuses installation and loading, disables an installed copy and names the reason. Never offer “run anyway” for failed provenance. An explicitly selected local source-development operation remains a separate informed code-execution decision; no automatic source fallback after refusal. A downloaded cache file has no authority merely because its filename or prior database row says “verified”.

## Compromise, revocation and rotation

A compromised mirror can substitute, truncate or withhold bytes; hashes and signatures prevent accepting substitutions, but cannot ensure availability. A compromised index can hide updates or present old valid attestations and misleading labels. It cannot invent signed source/build bindings without a trusted key. It can replay still-authorized artifacts: users select versions explicitly, and signed freshness metadata limits replay of revoked ones. Native source can itself be malicious even when every check succeeds.

Propose a separately signed trust snapshot, domain-separated from envelopes, with monotonic sequence, issue/expiry times (maximum seven days), revoked artifact/payload hashes, revoked key IDs and project-wide revocations. Bundle an offline policy root in the launcher; it authorizes builder keys and signs trust snapshots. Persist the highest sequence and refuse rollback; fresh installations depend on the bundled minimum sequence and current snapshot. Offline execution requires an unexpired cached snapshot; unavailable or stale metadata fails closed. A compromised registry cannot remove a previously authenticated revocation. Clock rollback must refuse freshness validation when time precedes the last accepted snapshot time. This still assumes a trustworthy local clock and launcher installation.

A stolen builder key can sign arbitrary malware with convincing provenance until revocation reaches launchers. Neither minisign nor recorded workflow IDs constrain what its holder signs. Keep keys out of untrusted build jobs; #802 should build without secrets and let a separate signing job validate successful immutable outputs and approved inputs. Failed or cancelled builds publish no signed usable record. Reproducibility is an acceptance check for #802, not a safety proof.

Rotate builder keys through root-signed authorization with bounded validity and explicit overlap, never through an index-provided key. Revoke a compromised key entirely, including old signatures, and rebuild/re-sign audited artifacts under a fresh key. Consent cannot override revocation. Root compromise requires a trusted launcher update replacing the root; an attacker controlling that update channel or the local account is outside this contract. Revocation stops future launches; it cannot undo code already executed.

## Acceptance matrix (future tests)

Use independently prepared fixture signatures and fixed expected hashes; these are proposed outcomes, not tests run by this note. Baseline: mod `fixture`, version `1.0.0`, build `build-A`, target `x86_64-unknown-linux-gnu`, source commit forty `a` characters, artifact bytes `abc` (size `3`, SHA-256 `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`), authorized key `registry-test-1`, fresh snapshot sequence `7`.

| Input/change from baseline | Expected value |
| --- | --- |
| Both valid registry signatures, exact identities, explicit consent | `Verified`, admit 3 bytes; execution allowed only after admission. |
| Author-key signatures, missing envelope, unknown schema or altered commit | `ProvenanceRefused`, zero executions and zero admitted files. |
| Signed build `build-B` or target `aarch64-apple-darwin` | `IdentityMismatch`, no architecture/build fallback. |
| Download `abd`, declared size 3 | `HashMismatch`, zero executions. |
| File size 67108865, payload 16385 bytes, duplicate JSON `build` | `BoundsOrFormatRefused`, no consent prompt. |
| Third-party release URL or redirect outside registry | `TransportRefused`, no binary download from destination. |
| Valid bytes, consent absent/cancelled | `ConsentRequired` / `Cancelled`, zero executions. |
| Cache replaced with `abd` after admission | `HashMismatch`, disable copy. |
| Snapshot 6 after accepting 7; expired snapshot offline | `TrustRollback` / `TrustExpired`, zero executions. |
| Revoked artifact, key or project despite recorded consent | `Revoked`, disable installed copy. |
| New authorized key, both signatures valid, fresh policy, renewed consent | `Verified`; index-only key addition gives `UnknownKey`. |
| #802 build fails or is cancelled | No signed envelope or selectable binary record published. |

## Proposed stages

1. **Contract (this note):** agree envelope bytes, key ownership and freshness policy with #802. File no additional issues.
2. **Producer:** #802 implements isolated builds, input recording, reproducible fixture and separate signing; publish independent fixtures for verifier review.
3. **Verifier:** #804 implements shared bounded parsing, trust snapshots, cache admission and consent, using the owners above. Remove any alternative unsigned acceptance path.
4. **Integration:** audit every loader and export route with fault injection for cancellation, cache replacement, offline expiry and revocation before enabling registry-native execution. No Rust changes are included here.

## Open questions

- Who holds the registry root and CI signing credentials? Recommend an offline maintainer root, separately scoped online builder keys, and no credentials in mod build jobs; actual identities and public keys are not yet provisioned here.
- Which exact encoding should #802 emit? Recommend exact signed UTF-8 bytes with the domain prefix and strict JSON above, shared fixtures and no canonicalization dependency. Approve the proposed schema before implementing either side.
- Is seven-day offline validity acceptable? Recommend that fail-closed window plus explicit refresh, with no consent override; product owners must accept the availability cost.
- Where does the installed game get its authoritative exact build ID? Recommend the trusted prebuilt manifest/loader identity used for #803 lookup, including archive identity in build inputs. Current lookup uses `revision`; do not assume that equals the eventual loader ABI identity without auditing #797 and release metadata.
- How are reviewed source-package identity and dependency inputs authenticated? Recommend binding them in root-authorized registry publication metadata and the signed input manifest, rejecting mutable refs. The existing unsigned index alone cannot establish reviewed ownership against registry compromise.
- Can every execution route retain immutable verified files? Recommend a loader-owned verified artifact handle and launch-time checks on every native dependency. Complete GUI/CLI/server/export/direct-loader census and OS-specific replacement protections remain implementation work.
