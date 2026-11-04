# Publishing your mod

## Publish your mod

Horizon's registry lists mods, modpacks (`.hpack`) and resource packs made for Horizon. **A project is reviewed
once, when it is listed; its later releases are checked by machine, not read by a person.** Players find them at
[playhorizon.dev/mods](https://playhorizon.dev/mods) and in the launcher (Mods, then **Horizon** beside Modrinth),
which checks every file before it installs it.

1. Run `cargo horizon-mod check` on the file (above): the registry runs the same checks.
2. Put the source in a public GitHub repository, under a licence that lets players run it. A native mod's `.hmod` is
   its source.
3. Publish a **GitHub release** in it with the file as an asset (`.hmod`, `.hpack`, `.hshader`, or a resource
   pack's `.zip`), and its `.minisig` beside it if you sign. The release notes are the changelog.
4. List the project once: open [playhorizon.dev/mods/submit](https://playhorizon.dev/mods/submit), sign in with GitHub
   and give the repository and the page (name, summary, licence, optionally a minisign public key). It opens a pull
   request in [horizon-rust/horizon-mods](https://github.com/horizon-rust/horizon-mods) adding `mods/<id>.toml`, with
   you as owner. A reviewer reads the repository and its source and merges it.

**Releasing a version is only step 3.** Publish a GitHub release (not a draft or a pre-release); within the hour the
registry reads it, runs the checks a submission runs (the file's own, the signature, the id and the version in the
file's manifest), and lists it. A release that fails is skipped and reported in an issue of the registry, never
published. Nobody opens a pull request and nobody reviews it. The version is the one in the file's manifest (the tag
when the file has none), the date and the changelog are the release's, the SHA-256 is computed by the registry, and
the launcher downloads the asset from your repository and refuses any other bytes. A published file never changes:
publish a new version. `revoked.toml` takes a bad one down.

Once it is published, its page and its card in the launcher show its rating: players signed in with GitHub on its
page give it 1 to 5 stars, one rating per account, which they can change. Its owners can't rate it, and nobody sees
who rated.

Your page can live in your repository too: a `horizon-registry.toml` at its root (on the newest release's tag) with
`name`, `summary`, `description`, `authors` and `license` fills what the registry's entry leaves out; the entry's own
words win. Its owners, key and icon are the registry's, changed by a pull request; its kind never changes. Hosted releases in an entry's
`versions` remain listed. The formats: [api/mod-index.md](../reference/registry-index.md).

### Integrity and signing

- **Integrity.** Every download is checked against the SHA-256 the index lists before the file reaches the game
  folder.
- **Signing.** Authors sign files with an Ed25519 key (minisign format: a `.minisig` beside the file); the index pins
  each project's public key, so an update signed by another key is refused. Unsigned local files stay allowed, with an
  "unsigned" mark.
- **Takedowns.** A file the registry takes down (`revoked.toml`) is refused by the launcher and turned off where
  installed.
