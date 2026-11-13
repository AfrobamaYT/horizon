//! The mod SDK's identity (design: `sdk.json`): one index per game build naming every asset of its
//! release with its checksum, size, kind and codec, the package layout number, and the files each
//! route needs before any download, build or packaging starts. Every reader of these bytes ends a
//! bad input in a named [`Refusal`], never a panic.
//!
//! Schema numbers are integers: a new field does not bump [`SCHEMA`], a removed or redefined one
//! does. Readers ignore fields they do not know and refuse a schema they do not know.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt,
    io::Read,
    path::{Path, PathBuf},
};

/// The index's file name, in the release and in an unpacked SDK folder.
pub const INDEX: &str = "sdk.json";
/// The index schema this reader knows.
pub const SCHEMA: u32 = 1;
/// The prebuilt package's layout number. Its one place is the file `sdk/package-format`, which
/// the archive name, the package's identity file, `sdk.json` and the scripts all read.
pub const PACKAGE_FORMAT: u32 = number(include_str!("sdk/package-format"));

/// A decimal number and a newline, at compile time: anything else fails the build.
const fn number(text: &str) -> u32 {
    let bytes = text.as_bytes();
    assert!(bytes.len() > 1 && bytes[bytes.len() - 1] == b'\n');
    let mut value = 0u32;
    let mut at = 0;
    while at < bytes.len() - 1 {
        assert!(
            bytes[at].is_ascii_digit(),
            "sdk/package-format holds one number"
        );
        value = value * 10 + (bytes[at] - b'0') as u32;
        at += 1;
    }
    value
}
/// The game version every build of this source implements.
pub const GAME: &str = horizon_core::version::VERSION_ID;
/// Files a release folder holds besides its assets: the index and its signed sums.
pub const BESIDE_ASSETS: [&str; 3] = [INDEX, "SHA256SUMS", "SHA256SUMS.sig"];

/// `sdk.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Index {
    pub schema: u32,
    pub build: Build,
    pub package_format: u32,
    pub toolchain: Toolchain,
    /// The content manifest's version ([`crate::manifest::API`]).
    pub content_api: String,
    pub surface: Surface,
    pub mixins: Mixins,
    /// Every asset of the release but the index itself, by file name.
    pub assets: BTreeMap<String, Asset>,
    /// The files each route needs, named before any work starts ([`inputs`]).
    pub inputs: Inputs,
}

/// Which game build an index describes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Build {
    /// The game's full commit sha: the build id.
    pub commit: String,
    /// The vanilla version ([`GAME`]).
    pub game: String,
    /// The commit's date, `YYYY-MM-DD` (UTC).
    pub date: String,
    /// The Nightly name that ships this build, when one does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nightly: Option<String>,
}

/// The Rust release everything in the SDK was compiled with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Toolchain {
    pub rust: String,
}

/// The stable surface (`surface.json`) this build offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Surface {
    /// The surface document's own `hash`: equal hashes, equal surface.
    pub hash: String,
}

/// The pinned horizon-mixins tool, which keeps its own release.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mixins {
    pub rev: String,
    /// The SHA-256 of the carried patch ([`crate::mixins_tool::patch_hash`]).
    pub patch: String,
}

/// One release asset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Asset {
    pub sha256: String,
    pub size: u64,
    pub kind: Kind,
    pub codec: Codec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<Side>,
}

/// What an asset holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// `surface.json`: what an API-only mod can name.
    Surface,
    /// `build-data.tar.xz`: `points.json`, `internals.json`, `loops.json`.
    BuildData,
    /// The API crates with their generated code written out.
    ApiSource,
    /// Game libraries, the prebuilt executable's inputs, the stub mods library.
    Runtime,
    /// Metadata of the API closure and its proc macros.
    Api,
    /// Every other compiled file. Until the package is sliced, the whole prebuilt game.
    Engine,
}

impl Kind {
    const ALL: [Kind; 6] = [
        Kind::Surface,
        Kind::BuildData,
        Kind::ApiSource,
        Kind::Runtime,
        Kind::Api,
        Kind::Engine,
    ];

    /// The name `sdk.json` writes.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Surface => "surface",
            Kind::BuildData => "build-data",
            Kind::ApiSource => "api-source",
            Kind::Runtime => "runtime",
            Kind::Api => "api",
            Kind::Engine => "engine",
        }
    }

    /// Whether its assets exist once per target (and side).
    pub fn per_target(self) -> bool {
        matches!(self, Kind::Runtime | Kind::Api | Kind::Engine)
    }
}

/// How an asset's bytes are packed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Codec {
    /// The file as it is.
    None,
    /// A tar archive, xz-compressed.
    Xz,
}

/// Which game a per-target asset is built for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Client,
    Server,
}

impl Asset {
    /// The kind, codec, target and side an asset's file name states:
    /// `surface.json`, `build-data.tar.xz`, `api-source.tar.xz` and
    /// `<runtime|api|engine>-<target>[-server].tar.xz`, where a target is made of `a-z`, `0-9`,
    /// `_` and `-` only. `None` for any other name, so a name this accepts is a plain file name.
    pub fn parts_of(name: &str) -> Option<(Kind, Codec, Option<String>, Option<Side>)> {
        if name == "surface.json" {
            return Some((Kind::Surface, Codec::None, None, None));
        }
        let stem = name.strip_suffix(".tar.xz")?;
        let kind = Kind::ALL.into_iter().find(|kind| {
            if kind.per_target() {
                stem.starts_with(&format!("{}-", kind.name()))
            } else {
                stem == kind.name()
            }
        })?;
        if !kind.per_target() {
            return Some((kind, Codec::Xz, None, None));
        }
        let rest = &stem[kind.name().len() + 1..];
        let (target, side) = match rest.strip_suffix("-server") {
            Some(target) => (target, Side::Server),
            None => (rest, Side::Client),
        };
        let plain = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-';
        (!target.is_empty() && target.chars().all(plain))
            .then(|| (kind, Codec::Xz, Some(target.to_owned()), Some(side)))
    }

    /// A per-target asset's file name, as [`Asset::parts_of`] reads it.
    pub fn name(kind: Kind, target: &str, side: Side) -> String {
        let suffix = if side == Side::Server { "-server" } else { "" };
        format!("{}-{target}{suffix}.tar.xz", kind.name())
    }
}

/// The files each route needs: the one list the packager, `developer-install`, the launcher and
/// the mod tool read (`sdk/inputs.json`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inputs {
    pub routes: BTreeMap<String, Route>,
    /// How to make each listed file, by its name.
    pub how: BTreeMap<String, String>,
}

/// What one route needs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    /// Files beside the game program the route starts from. A name with no extension is a
    /// program and takes the game program's executable suffix.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub program: Vec<String>,
    /// Files in the game checkout, relative to its root.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checkout: Vec<String>,
    /// SDK asset kinds, one asset each for the host's target and side.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sdk: Vec<Kind>,
}

/// The required inputs of every route.
pub fn inputs() -> Inputs {
    serde_json::from_str(include_str!("sdk/inputs.json")).expect("sdk/inputs.json is an input list")
}

impl Inputs {
    /// The route named `name`. The list may come from a downloaded `sdk.json`, so an unknown
    /// name is a refusal.
    pub fn route(&self, name: &str) -> Result<&Route, Refusal> {
        self.routes
            .get(name)
            .ok_or_else(|| Refusal::Route(name.to_owned()))
    }

    /// How to make `name` (as listed, without `.exe`) for the game program `program`: `PROFILE`
    /// becomes the cargo flag of the profile `program` was built with (its folder's name) and
    /// `PROGRAM_FOLDER` its folder, so the command makes the file where the check looks.
    pub fn how(&self, name: &str, program: &Path) -> Option<String> {
        let folder = program.parent().unwrap_or(Path::new(""));
        let profile = match folder.file_name().and_then(|n| n.to_str()) {
            Some("debug") | None => String::new(),
            Some("release") => "--release".to_owned(),
            Some(other) => format!("--profile {other}"),
        };
        let how = self.how.get(name)?;
        let how = how.replace("PROFILE", &profile).replace("  ", " ");
        Some(how.replace("PROGRAM_FOLDER", &folder.display().to_string()))
    }

    /// Every file of `route` that is missing, beside `program` or in `checkout`, together with
    /// how to make each, in one message; `Ok` when nothing is missing.
    pub fn check(&self, route: &str, program: &Path, checkout: &Path) -> Result<(), Refusal> {
        let route = self.route(route)?;
        let missing: Vec<String> = route
            .program_files(program)
            .into_iter()
            .map(|path| {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                (name.into_owned(), path)
            })
            .chain(
                route
                    .checkout
                    .iter()
                    .map(|name| (name.clone(), checkout.join(name))),
            )
            .filter(|(_, path)| !path.is_file())
            .map(|(name, _)| name)
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        let how = missing
            .iter()
            .filter_map(|name| {
                let listed = name.strip_suffix(".exe").unwrap_or(name);
                self.how(listed, program)
                    .map(|how| format!("{name}: {how}"))
            })
            .collect();
        Err(Refusal::MissingInputs { missing, how })
    }
}

impl Route {
    /// The route's files beside `program`, with its executable suffix on programs.
    pub fn program_files(&self, program: &Path) -> Vec<PathBuf> {
        let exe = program.extension().is_some_and(|e| e == "exe");
        self.program
            .iter()
            .map(|name| {
                let name = if exe && !name.contains('.') {
                    format!("{name}.exe")
                } else {
                    name.clone()
                };
                program.with_file_name(name)
            })
            .collect()
    }
}

/// Why an SDK index, asset or input set was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Refusal {
    /// The index is not JSON of the expected form.
    Malformed(String),
    /// The index's schema is one this reader does not know.
    Schema(u64),
    /// The index describes another build than the one asked for.
    Build { expected: String, found: String },
    /// The package layout is one this reader does not know.
    PackageFormat(u64),
    /// An asset names a kind this reader does not know.
    Kind { asset: String, kind: String },
    /// An asset names a codec this reader does not know.
    Codec { asset: String, codec: String },
    /// An asset's name is not a plain file name, or does not say what the entry claims.
    AssetName(String),
    /// Listed assets that are not there.
    Missing(Vec<String>),
    /// A file that the index does not list.
    Unlisted(String),
    /// An asset's size differs from the index's.
    Size {
        asset: String,
        expected: u64,
        found: u64,
    },
    /// An asset's SHA-256 differs from the index's.
    Checksum { asset: String },
    /// An asset unpacks to more than its limit.
    TooLarge { asset: String, limit: u64 },
    /// An asset's archive is damaged or holds something other than files and folders.
    Archive { asset: String, why: String },
    /// Reading or writing failed.
    Io(String),
    /// The input list has no such route.
    Route(String),
    /// Files a route needs are not there; how to make each.
    MissingInputs {
        missing: Vec<String>,
        how: Vec<String>,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(why) => write!(f, "{INDEX} is not a valid SDK index: {why}"),
            Self::Schema(schema) => write!(
                f,
                "{INDEX} has schema {schema}; this program reads schema {SCHEMA}. Update it to use this SDK"
            ),
            Self::Build { expected, found } => {
                write!(f, "{INDEX} is for build {found}, not {expected}")
            }
            Self::PackageFormat(format) => write!(
                f,
                "{INDEX} has package format {format}; this program reads format {PACKAGE_FORMAT}. Update it to use this SDK"
            ),
            Self::Kind { asset, kind } => write!(f, "{asset}: unknown asset kind {kind:?}"),
            Self::Codec { asset, codec } => write!(f, "{asset}: unknown codec {codec:?}"),
            Self::AssetName(asset) => write!(f, "{asset:?} is not a valid SDK asset name"),
            Self::Missing(assets) => write!(f, "missing SDK assets: {}", assets.join(", ")),
            Self::Unlisted(asset) => write!(f, "{asset} is not listed in {INDEX}"),
            Self::Size {
                asset,
                expected,
                found,
            } => write!(
                f,
                "{asset} is {found} bytes; {INDEX} lists {expected}. Download it again"
            ),
            Self::Checksum { asset } => write!(
                f,
                "{asset} does not match its checksum in {INDEX}. Download it again"
            ),
            Self::TooLarge { asset, limit } => {
                write!(f, "{asset} unpacks to more than {limit} bytes")
            }
            Self::Archive { asset, why } => write!(f, "{asset}: damaged archive: {why}"),
            Self::Io(why) => f.write_str(why),
            Self::Route(route) => write!(f, "the SDK's input list has no route {route}"),
            Self::MissingInputs { missing, how } => {
                write!(f, "missing required inputs: {}", missing.join(", "))?;
                for line in how {
                    write!(f, "\n  {line}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for Refusal {}

/// The index as written, before its kinds and codecs are checked.
#[derive(Deserialize)]
struct Written {
    build: Build,
    package_format: u64,
    toolchain: Toolchain,
    content_api: String,
    surface: Surface,
    mixins: Mixins,
    assets: BTreeMap<String, WrittenAsset>,
    inputs: Inputs,
}

#[derive(Deserialize)]
struct WrittenAsset {
    sha256: String,
    size: u64,
    kind: String,
    codec: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    side: Option<Side>,
}

impl Index {
    /// The index of the release folder `folder` for `build`, compiled with Rust `rust`: every file
    /// there but [`BESIDE_ASSETS`] becomes an asset, named as [`Asset::parts_of`] reads it, and the
    /// surface hash comes from its `surface.json`. The publish step writes it ([`Index::to_json`]).
    pub fn of_folder(folder: &Path, build: Build, rust: String) -> Result<Index, Refusal> {
        let io = |error: std::io::Error| Refusal::Io(format!("{}: {error}", folder.display()));
        let mut assets = BTreeMap::new();
        for entry in std::fs::read_dir(folder).map_err(io)? {
            let name = entry
                .map_err(io)?
                .file_name()
                .to_string_lossy()
                .into_owned();
            if BESIDE_ASSETS.contains(&name.as_str()) {
                continue;
            }
            let (kind, codec, target, side) =
                Asset::parts_of(&name).ok_or_else(|| Refusal::AssetName(name.clone()))?;
            let mut file = std::fs::File::open(folder.join(&name)).map_err(io)?;
            let mut hash = Sha256::new();
            let size = std::io::copy(&mut file, &mut hash).map_err(io)?;
            let asset = Asset {
                sha256: format!("{:x}", hash.finalize()),
                size,
                kind,
                codec,
                target,
                side,
            };
            assets.insert(name, asset);
        }
        #[derive(Deserialize)]
        struct Document {
            hash: String,
        }
        let surface = std::fs::read(folder.join("surface.json"))
            .map_err(|_| Refusal::Missing(vec!["surface.json".into()]))?;
        let Document { hash } = serde_json::from_slice(&surface)
            .map_err(|error| Refusal::Malformed(format!("surface.json: {error}")))?;
        Ok(Index {
            schema: SCHEMA,
            build,
            package_format: PACKAGE_FORMAT,
            toolchain: Toolchain { rust },
            content_api: crate::manifest::API.to_owned(),
            surface: Surface { hash },
            mixins: Mixins {
                rev: crate::mixins_tool::REV.to_owned(),
                patch: crate::mixins_tool::patch_hash(),
            },
            assets,
            inputs: inputs(),
        })
    }

    /// `sdk.json`'s text: pretty, sorted, ending in a newline.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("an index serializes") + "\n"
    }

    /// Read `sdk.json` for build `commit`: its schema, build, package format and every asset's
    /// kind, codec and name are checked here, once.
    pub fn read(bytes: &[u8], commit: &str) -> Result<Index, Refusal> {
        #[derive(Deserialize)]
        struct Schema {
            schema: u64,
        }
        let malformed = |error: serde_json::Error| Refusal::Malformed(error.to_string());
        let Schema { schema } = serde_json::from_slice(bytes).map_err(malformed)?;
        if schema != u64::from(SCHEMA) {
            return Err(Refusal::Schema(schema));
        }
        let written: Written = serde_json::from_slice(bytes).map_err(malformed)?;
        if written.build.commit != commit {
            return Err(Refusal::Build {
                expected: commit.to_owned(),
                found: written.build.commit,
            });
        }
        if written.package_format != u64::from(PACKAGE_FORMAT) {
            return Err(Refusal::PackageFormat(written.package_format));
        }
        let mut assets = BTreeMap::new();
        for (name, asset) in written.assets {
            let kind = Kind::ALL
                .into_iter()
                .find(|kind| kind.name() == asset.kind)
                .ok_or_else(|| Refusal::Kind {
                    asset: name.clone(),
                    kind: asset.kind.clone(),
                })?;
            let codec = match asset.codec.as_str() {
                "none" => Codec::None,
                "xz" => Codec::Xz,
                _ => {
                    return Err(Refusal::Codec {
                        asset: name,
                        codec: asset.codec,
                    });
                }
            };
            let entry = Asset {
                sha256: asset.sha256,
                size: asset.size,
                kind,
                codec,
                target: asset.target,
                side: asset.side,
            };
            // The name is a plain file name that says what the entry claims, so it is safe to
            // join to a folder and to build from a kind, target and side.
            if Asset::parts_of(&name) != Some((kind, codec, entry.target.clone(), entry.side)) {
                return Err(Refusal::AssetName(name));
            }
            assets.insert(name, entry);
        }
        Ok(Index {
            schema: SCHEMA,
            build: written.build,
            package_format: PACKAGE_FORMAT,
            toolchain: written.toolchain,
            content_api: written.content_api,
            surface: written.surface,
            mixins: written.mixins,
            assets,
            inputs: written.inputs,
        })
    }

    /// The listed asset `name`, or why there is none.
    pub fn asset(&self, name: &str) -> Result<&Asset, Refusal> {
        self.assets
            .get(name)
            .ok_or_else(|| Refusal::Unlisted(name.to_owned()))
    }

    /// The asset names `route` needs on `target` and `side`, refusing with every missing one.
    pub fn route_assets(
        &self,
        route: &str,
        target: &str,
        side: Side,
    ) -> Result<Vec<String>, Refusal> {
        let route = self.inputs.route(route)?;
        let names: Vec<String> = route
            .sdk
            .iter()
            .map(|&kind| Asset::name(kind, target, side))
            .collect();
        let missing: Vec<String> = names
            .iter()
            .filter(|name| !self.assets.contains_key(*name))
            .cloned()
            .collect();
        if missing.is_empty() {
            Ok(names)
        } else {
            Err(Refusal::Missing(missing))
        }
    }

    /// Check `body` against the listed asset `name`: its size, then its SHA-256. Reads at most
    /// one byte past the listed size.
    pub fn verify(&self, name: &str, body: impl Read) -> Result<(), Refusal> {
        let asset = self.asset(name)?;
        let mut body = body.take(asset.size.saturating_add(1));
        let mut hash = Sha256::new();
        let mut size = 0u64;
        let mut buffer = [0u8; 1 << 16];
        loop {
            let read = body
                .read(&mut buffer)
                .map_err(|error| Refusal::Io(format!("{name}: {error}")))?;
            if read == 0 {
                break;
            }
            size += read as u64;
            hash.update(&buffer[..read]);
        }
        if size != asset.size {
            return Err(Refusal::Size {
                asset: name.to_owned(),
                expected: asset.size,
                found: size,
            });
        }
        if format!("{:x}", hash.finalize()) != asset.sha256 {
            return Err(Refusal::Checksum {
                asset: name.to_owned(),
            });
        }
        Ok(())
    }

    /// Check a release folder: every listed asset is there and matches, and nothing else is
    /// there but the index and its sums ([`BESIDE_ASSETS`]).
    pub fn verify_folder(&self, folder: &Path) -> Result<(), Refusal> {
        let io = |error: std::io::Error| Refusal::Io(format!("{}: {error}", folder.display()));
        let mut present = Vec::new();
        for entry in std::fs::read_dir(folder).map_err(io)? {
            let name = entry.map_err(io)?.file_name();
            let name = name.to_string_lossy().into_owned();
            if !BESIDE_ASSETS.contains(&name.as_str()) {
                self.asset(&name)?;
                present.push(name);
            }
        }
        let missing: Vec<String> = self
            .assets
            .keys()
            .filter(|name| !present.contains(name))
            .cloned()
            .collect();
        if !missing.is_empty() {
            return Err(Refusal::Missing(missing));
        }
        for name in self.assets.keys() {
            let file = std::fs::File::open(folder.join(name)).map_err(io)?;
            self.verify(name, file)?;
        }
        Ok(())
    }
}

/// The largest xz dictionary [`unpack`] decodes with: `xz -9`'s. The decoder's memory is this plus
/// fixed buffers, whatever the archive holds; an archive asking for more is refused as damaged.
#[cfg(feature = "prebuilt")]
pub const DICTIONARY_LIMIT: usize = 64 << 20;

/// Unpack the listed archive `name`, downloaded to the file `file`, into the new folder `into`.
/// The file is checked against the index first ([`Index::verify`]), so nothing unverified is
/// decoded. The archive streams through a bounded dictionary into a folder beside `into`, which is
/// renamed into place only when every entry unpacked: a refusal partway leaves nothing. Refused:
/// unpacked bytes past `limit` (a decompression bomb), and any entry that is not a plain file or
/// folder inside the destination.
#[cfg(feature = "prebuilt")]
pub fn unpack(
    index: &Index,
    name: &str,
    file: &Path,
    into: &Path,
    limit: u64,
) -> Result<(), Refusal> {
    let open = || {
        std::fs::File::open(file)
            .map_err(|error| Refusal::Io(format!("{}: {error}", file.display())))
    };
    index.verify(name, open()?)?;
    if index.asset(name)?.codec != Codec::Xz {
        return Err(Refusal::Archive {
            asset: name.to_owned(),
            why: "not an archive".into(),
        });
    }
    let io = |error: std::io::Error| Refusal::Io(format!("{}: {error}", into.display()));
    let parent = into
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let partial = Partial::new(parent).map_err(io)?;
    extract(name, open()?, &partial.0, limit)?;
    // Once renamed, the partial folder's removal on drop finds nothing.
    std::fs::rename(&partial.0, into).map_err(io)
}

/// Decode the xz tar `body` into the existing folder `into`, counting the decoded bytes as they
/// are read.
#[cfg(feature = "prebuilt")]
fn extract(name: &str, body: impl Read + 'static, into: &Path, limit: u64) -> Result<(), Refusal> {
    use std::{cell::Cell, rc::Rc};
    /// A reader that fails once more than `limit` bytes came through it.
    struct Capped<R> {
        inner: R,
        read: Rc<Cell<u64>>,
        limit: u64,
    }
    impl<R: Read> Read for Capped<R> {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let read = self.inner.read(buffer)?;
            self.read.set(self.read.get() + read as u64);
            if self.read.get() > self.limit {
                return Err(std::io::Error::other("over the limit"));
            }
            Ok(read)
        }
    }
    let damaged = |why: String| Refusal::Archive {
        asset: name.to_owned(),
        why,
    };
    let decoder = xz4rust::XzDecoder::in_heap_with_alloc_dict_size(1 << 20, DICTIONARY_LIMIT);
    let buffer = std::num::NonZeroUsize::new(1 << 16).expect("non-zero");
    let read = Rc::new(Cell::new(0));
    let capped = Capped {
        inner: xz4rust::XzReader::new_with_buffer_size_and_decoder(body, buffer, decoder),
        read: read.clone(),
        limit,
    };
    let over = |error: std::io::Error| {
        if read.get() > limit {
            Refusal::TooLarge {
                asset: name.to_owned(),
                limit,
            }
        } else {
            damaged(error.to_string())
        }
    };
    let mut archive = tar::Archive::new(capped);
    for entry in archive.entries().map_err(over)? {
        let mut entry = entry.map_err(over)?;
        let path = entry.path().map_err(over)?.into_owned();
        let kind = entry.header().entry_type();
        if !kind.is_file() && !kind.is_dir() {
            return Err(damaged(format!(
                "{} is not a file or folder",
                path.display()
            )));
        }
        if !entry.unpack_in(into).map_err(over)? {
            return Err(damaged(format!("{} leaves the folder", path.display())));
        }
    }
    Ok(())
}

/// A new folder beside the destination, removed when dropped.
#[cfg(feature = "prebuilt")]
struct Partial(PathBuf);

#[cfg(feature = "prebuilt")]
impl Partial {
    fn new(parent: &Path) -> std::io::Result<Self> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = parent.join(format!(
            ".sdk-unpack.{}.{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

#[cfg(feature = "prebuilt")]
impl Drop for Partial {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
#[path = "sdk/tests.rs"]
mod tests;
