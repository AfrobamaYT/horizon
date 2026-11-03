//! Mods in the game (`docs/mods.md`): content mods' blocks, items, data and
//! assets (`content_root`), and the Mods page's rows. Code mods are native
//! (`native_hooks.rs`); executable components in content packages are refused.
use super::{Game, execution_role::ExecutionRole};
use crate::mod_api::{RegistrationContext, RegistrationPhase};
use horizon_client::mod_list::{self, Installed};
use std::path::PathBuf;

#[derive(Clone, Default)]
pub(super) struct ModsState {
    /// `--dev-mod PATH`: content mods always on.
    pub(super) dev: Vec<PathBuf>,
    pub(super) native: std::sync::Arc<horizon_mods::runtime::Runtime>,
}

/// Freeze native component IDs before reading content or opening a connection.
/// With no mod registering any, nothing is installed, which every reader
/// treats as no custom components, and `registries.json` isn't read.
pub(super) fn register_components(root: &std::path::Path) -> horizon_client::Result<()> {
    let mut names = Vec::new();
    Game::native_component_types(
        &RegistrationContext::new(RegistrationPhase::Components),
        &mut names,
    );
    if names.is_empty() {
        return Ok(());
    }
    let path =
        horizon_protocol::registry::Builtin::find(root).ok_or("registries.json not found")?;
    let builtin = horizon_protocol::registry::Builtin::load(&path)?;
    let types = builtin
        .get("minecraft:data_component_type")
        .ok_or("missing data component registry")?;
    let first = i32::try_from(types.names.len())?;
    horizon_protocol::item::custom::install(first, names)
        .map_err(|error| format!("invalid native component registration: {error}"))?;
    Ok(())
}

struct NativeDeclared {
    blocks: Vec<horizon_mods::content::Block>,
    items: Vec<horizon_mods::content::Item>,
    types: Vec<horizon_mods::content::BlockEntityType>,
}

/// The same native declarations serve startup and every pack reload.
fn native_declared() -> &'static NativeDeclared {
    static DECLARED: std::sync::OnceLock<NativeDeclared> = std::sync::OnceLock::new();
    DECLARED.get_or_init(|| {
        let (mut blocks, mut items) = (Vec::new(), Vec::new());
        Game::native_blocks(
            &RegistrationContext::new(RegistrationPhase::Blocks),
            &mut blocks,
        );
        Game::native_items(
            &RegistrationContext::new(RegistrationPhase::Items),
            &mut items,
        );
        let types = crate::mod_api::block_entity_types().to_vec();
        NativeDeclared {
            blocks,
            items,
            types,
        }
    })
}
/// The native mods' standalone models (`native_standalone_models`), set once
/// before the resources first load.
pub(super) fn register_standalone_models(role: ExecutionRole) {
    if !role.is_client() {
        return;
    }
    let mut names = Vec::new();
    Game::native_standalone_models(
        &RegistrationContext::new(RegistrationPhase::StandaloneModels),
        &mut names,
    );
    horizon_client::models::register_standalone(names);
}

/// `root` with the content mods that are on layered over it
/// (docs/modpacks.md): their blocks, items, data and assets. Read once at
/// start, before the resources load; a mod whose content is refused is left
/// out here and its row on the Mods page says why.
pub(super) fn content_root(
    root: &std::path::Path,
    enabled: &std::collections::BTreeSet<String>,
    sources: &ModsState,
) -> horizon_client::Result<std::path::PathBuf> {
    use horizon_client::mod_content::{self, ContentMod};
    use horizon_mods::Content;
    let here = std::env::current_dir().unwrap_or_default();
    let content: Vec<_> = horizon_mods::selection::content_packages(&here, enabled, &sources.dev)
        .filter(|selected| {
            !sources
                .native
                .built()
                .iter()
                .any(|built| built.id == selected.package.manifest.id)
        })
        .collect();
    let mut known = None;
    let mut mods: Vec<ContentMod> = Vec::new();
    let announce = !horizon_protocol::mod_list::is_set();
    let mut announced: Vec<_> = if announce {
        sources
            .native
            .built()
            .iter()
            .map(|m| announced_entry(m.id, m.version, m.side))
            .collect()
    } else {
        Vec::new()
    };
    struct PackageSource<'a> {
        path: &'a std::path::Path,
        manifest: &'a horizon_mods::Manifest,
        content_only: bool,
    }
    let native = sources.native.packages().iter().map(|p| PackageSource {
        path: &p.path,
        manifest: &p.manifest,
        content_only: false,
    });
    let content = content.iter().map(|p| PackageSource {
        path: &p.path,
        manifest: &p.package.manifest,
        content_only: true,
    });
    for source in native.chain(content) {
        let m = source.manifest;
        if mods.iter().any(|c| c.id == m.id) {
            continue;
        }
        // The items this mod's native code declares may be a file item's crafting remainder.
        let declared: Vec<String> = native_declared()
            .items
            .iter()
            .filter(|i| i.id.split_once(':').is_some_and(|(ns, _)| ns == m.id))
            .map(|i| i.id.clone())
            .collect();
        let content = match Content::read_declaring(source.path, m, &declared) {
            Ok(content) => content,
            Err(e) => {
                log::warn!("Horizon: mod {} content not loaded: {e}", m.id);
                continue;
            }
        };
        let known = match &mut known {
            Some(types) => types,
            None => match mod_content::known_components(root) {
                Ok(types) => known.insert(types),
                Err(e) => {
                    log::warn!("Horizon: content mods not loaded: {e}");
                    return Err(e);
                }
            },
        };
        let candidate = ContentMod {
            id: m.id.clone(),
            content,
        };
        match mod_content::accept_mod(&mut mods, candidate, known) {
            Ok(_) if announce && source.content_only => {
                announced.push(announced_entry(&m.id, &m.version, m.side))
            }
            Ok(_) => {}
            Err(e) => log::warn!("Horizon: mod {} content not loaded: {e}", m.id),
        }
    }
    let NativeDeclared {
        blocks,
        items,
        types,
    } = native_declared();
    if !blocks.is_empty() || !items.is_empty() || !types.is_empty() {
        let known = match &mut known {
            Some(types) => types,
            None => known.insert(mod_content::known_components(root)?),
        };
        let errors = mod_content::accept_declared(&mut mods, blocks, items, types, known);
        if !errors.is_empty() {
            return Err(errors.join("\n").into());
        }
        // A file item's remainder named a code-declared item that was refused: that mod
        // alone is left out, not every content mod through the overlay's check.
        for e in mod_content::drop_dangling_remainders(&mut mods) {
            log::warn!("Horizon: {e}");
        }
    }
    let out = mod_content::resource_root(root, &mods, &here.join(".horizon-cache"))?;
    register_block_entity_types(&out, &mods)?;
    if announce {
        horizon_protocol::mod_list::set_local(announced);
    }
    if out != root {
        log::info!(
            "Horizon: content from {} mod(s) in {}",
            mods.len(),
            out.display()
        );
    }
    register_mob_effects(&mods);
    register_entity_types(&mods);
    register_menu_types(&mods);
    horizon_client::mod_content::set_fluid_looks(&mods);
    Ok(out)
}

fn register_block_entity_types(
    root: &std::path::Path,
    mods: &[horizon_client::mod_content::ContentMod],
) -> horizon_client::Result<()> {
    use horizon_client::containers::block_entity_types::{self, BlockEntityTypes};
    use horizon_protocol::registry::Builtin;
    let path = Builtin::find(root).ok_or("registries.json not found")?;
    let builtin = Builtin::load(&path)?;
    let names = &builtin
        .get("minecraft:block_entity_type")
        .ok_or("missing block entity type registry")?
        .names;
    let membership = mods
        .iter()
        .flat_map(|m| &m.content.block_entity_types)
        .flat_map(|kind| {
            kind.blocks
                .iter()
                .map(|block| (block.clone(), kind.id.clone()))
        })
        .collect();
    let types = BlockEntityTypes::new(membership, names.clone());
    crate::mod_api::validate_block_entity_type_keys(&types)?;
    block_entity_types::install(types)?;
    Ok(())
}

/// Manifest identity for the configuration-phase mod check.
fn announced_entry(
    id: &str,
    version: &str,
    side: horizon_mods::Side,
) -> horizon_protocol::mod_list::Entry {
    use horizon_mods::Side;
    use horizon_protocol::mod_list;
    mod_list::Entry {
        id: id.to_owned(),
        version: version.to_owned(),
        side: match side {
            Side::Both => mod_list::Side::Both,
            Side::Client => mod_list::Side::Client,
        },
    }
}

/// The mods' mob effects join vanilla's (`brewing::effect`), once the root
/// holding their network ids is built. The startup root's effects, none
/// included, stay until restart: a pack reload after a Mods switch builds a
/// root with others.
fn register_mob_effects(mods: &[horizon_client::mod_content::ContentMod]) {
    use horizon_client::brewing::{self, Category};
    use horizon_mods::content::EffectCategory;
    let effects = horizon_client::mod_content::mob_effects(mods);
    let ids: Vec<String> = effects.iter().map(|e| e.id.clone()).collect();
    let set = brewing::register_mod_effects(effects.into_iter().map(|e| {
        let category = match e.category {
            EffectCategory::Beneficial => Category::Beneficial,
            EffectCategory::Harmful => Category::Harmful,
            EffectCategory::Neutral => Category::Neutral,
        };
        (e.id.clone(), e.color, category)
    }));
    if !set
        && brewing::mod_effects()
            .iter()
            .map(|e| e.id)
            .ne(ids.iter().map(String::as_str))
    {
        log::warn!("Horizon: the mods' mob effects changed; restart the game to use them");
    }
}

/// the mods' thrown entity types, as their mob effects: set once, from the
/// startup root.
fn register_entity_types(mods: &[horizon_client::mod_content::ContentMod]) {
    use horizon_client::projectiles;
    let types = horizon_client::mod_content::entity_types(mods);
    let set = projectiles::register_thrown_types(
        types
            .iter()
            .map(|t| (t.id.clone(), t.item.clone(), t.damage)),
    );
    if !set
        && projectiles::thrown_types()
            .iter()
            .map(|t| t.id)
            .ne(types.iter().map(|t| t.id.as_str()))
    {
        log::warn!("Horizon: the mods' entity types changed; restart the game to use them");
    }
}

/// the mods' menu types, as their entity types: set once, from the startup
/// root.
fn register_menu_types(mods: &[horizon_client::mod_content::ContentMod]) {
    use horizon_client::containers;
    let menus = horizon_client::mod_content::menu_types(mods);
    let set = containers::register_mod_menu_types(menus.iter().map(|m| {
        let results = m.results.iter().map(|&r| usize::from(r)).collect();
        (m.id.clone(), m.slots.into(), m.data_slots.into(), results)
    }));
    if !set
        && containers::mod_menu_types()
            .iter()
            .map(|m| m.id)
            .ne(menus.iter().map(|m| m.id.as_str()))
    {
        log::warn!("Horizon: the mods' menu types changed; restart the game to use them");
    }
}

/// The embedded inventory, formatted for the crash report.
fn native_mods_detail(built: &[horizon_mods::runtime::BuiltMod]) -> Option<String> {
    let list = built
        .iter()
        .map(|m| format!("{} {}", m.id, m.version))
        .collect::<Vec<_>>()
        .join(", ");
    (!list.is_empty()).then_some(list)
}

/// Installed native mods not compiled into this executable get an explanatory row.
fn not_built(
    dir: &std::path::Path,
    built: &[horizon_mods::runtime::BuiltMod],
    rows: &[Installed],
) -> Vec<Installed> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    paths.retain(|p| p.extension().is_some_and(|e| e == "hmod"));
    paths.sort();
    let mut out: Vec<Installed> = Vec::new();
    for path in paths {
        let Ok(bundle) = horizon_mods::index::native::Bundle::read(&path) else {
            continue;
        };
        let m = bundle.manifest;
        let listed = rows.iter().chain(&out).any(|row| row.id == m.id);
        if !listed && !built.iter().any(|b| b.id == m.id) {
            out.push(Installed {
                title: format!("{} {}", m.id, m.version),
                id: m.id,
                description: String::new(),
                problem: Some(mod_list::NOT_BUILT.to_owned()),
                dev: false,
                native: None,
            });
        }
    }
    out
}

impl Game<'_> {
    /// Lists the installed mods on the Mods page. Content mods load with the resources
    /// (`content_root`); native ones are built into the game (`native_hooks.rs`). Unsupported
    /// files get the same unreadable-file row as any other invalid package.
    pub(super) fn start_mods(&mut self) {
        let dir = Self::game_dir().join(mod_list::DIR);
        let mut paths = horizon_mods::find(&dir);
        paths.retain(|p| !self.mods.dev.contains(p));
        paths.extend(self.mods.dev.iter().cloned());
        self.settings.mods.installed = paths
            .iter()
            .map(|path| {
                let file = path.file_name().unwrap_or_default().to_string_lossy();
                let package = match horizon_mods::Package::read(path) {
                    Ok(package) => package,
                    Err(e) => {
                        return Installed {
                            id: String::new(),
                            title: file.into_owned(),
                            description: String::new(),
                            problem: Some(format!("Can't be read: {e}")),
                            dev: false,
                            native: None,
                        };
                    }
                };
                let m = &package.manifest;
                Installed {
                    id: m.id.clone(),
                    title: format!("{} {}", m.name, m.version),
                    description: m.description.clone(),
                    problem: None,
                    dev: self.mods.dev.contains(path),
                    native: None,
                }
            })
            .collect();
        // A mod with a native version built in says so instead.
        self.list_native_mods();
        let built = self.mods.native.built();
        let built_mods = native_mods_detail(built);
        if let Some(list) = built_mods {
            horizon_client::crash::set_detail("Native Mods", list);
        }

        let native_dir = Self::game_dir().join(mod_list::NATIVE_DIR);
        let unbuilt = not_built(&native_dir, built, &self.settings.mods.installed);
        self.settings.mods.installed.extend(unbuilt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_mods::runtime::{BuiltMod, Runtime};

    const BUILT: &[BuiltMod] = &[BuiltMod {
        id: "machine_sample",
        version: "0.3.0",
        side: horizon_mods::Side::Both,
    }];

    struct Fixture(PathBuf);
    impl Fixture {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("horizon-game-mods-{}-{label}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn runtime(&self) -> Runtime {
            let dir = self.0.join("compiled");
            let package = dir.join(mod_list::NATIVE_DIR).join("machine_sample");
            let file = std::fs::File::open(self.0.join("machine_sample.hmod")).unwrap();
            zip::ZipArchive::new(file)
                .unwrap()
                .extract(&package)
                .unwrap();
            Runtime::load(&dir.join("horizon-engine"), BUILT).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn standalone_models_register_only_on_the_client() {
        use crate::mod_api::{self, Hooks, ModEntry, StandaloneModelsEvent, testing::with_mods};
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        #[derive(Default)]
        struct Models(Arc<AtomicUsize>);
        #[crate::mod_api::horizon_mod]
        impl Hooks for Models {
            fn standalone_models(
                &self,
                context: &RegistrationContext<'_>,
                event: &mut StandaloneModelsEvent<'_>,
            ) {
                assert_eq!(context.phase(), RegistrationPhase::StandaloneModels);
                assert_eq!(
                    context.mods().ids().collect::<Vec<_>>(),
                    ["registration_test"]
                );
                self.0.fetch_add(1, Ordering::Relaxed);
                event.models.push("registration_test:block/shaft".into());
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        with_mods(
            vec![ModEntry::new(
                "registration_test",
                Box::new(Models(calls.clone())),
            )]
            .into(),
            || {
                register_standalone_models(ExecutionRole::IntegratedServer);
                register_standalone_models(ExecutionRole::DedicatedServer);
                assert_eq!(calls.load(Ordering::Relaxed), 0);
                register_standalone_models(ExecutionRole::Client);
                assert_eq!(calls.load(Ordering::Relaxed), 1);
                assert!(
                    horizon_client::models::standalone()
                        .iter()
                        .any(|id| id == "registration_test:block/shaft")
                );
            },
        );
    }

    fn hmod(dir: &std::path::Path, id: &str) {
        let manifest = format!(
            "id = '{id}'\nname = '{id}'\nversion = '0.3.0'\napi = '0.2'\nside = 'both'\ncapabilities = ['registries']\n[native]\ngame = '26.3'\nhorizon = '{}'\nmixins = '{}'\n",
            "a".repeat(40),
            "b".repeat(40)
        );
        let files: &[(&str, &[u8])] = &[
            (
                "Cargo.toml",
                b"[package]\nname='different_crate'\nversion='9.9.9'\n",
            ),
            ("src/lib.rs", b"pub fn hooks() {}"),
        ];
        let bytes = horizon_mods::Package::write_files(&manifest, files).unwrap();
        std::fs::write(dir.join(format!("{id}.hmod")), bytes).unwrap();
    }

    #[test]
    fn native_content_follows_the_build_and_misplaced_content_follows_its_selection() {
        const MODE: &str = "HORIZON_MODS_SELECTION_TEST";
        let mode = std::env::var(MODE).ok();
        if mode.is_none() {
            // Inventory and content registries are process-wide, as in a real startup.
            for mode in ["compiled", "vanilla"] {
                let status = std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", "game::mods::tests::native_content_follows_the_build_and_misplaced_content_follows_its_selection", "--nocapture"])
                    .env(MODE, mode).status().unwrap();
                assert!(status.success());
            }
            return;
        }
        let f = Fixture::new(mode.as_deref().unwrap());
        hmod(&f.0, "machine_sample");
        let native = if mode.as_deref() == Some("compiled") {
            f.runtime()
        } else {
            Runtime::default()
        };
        let here = f.0.join("unrelated-game");
        let misplaced = here.join(mod_list::NATIVE_DIR);
        std::fs::create_dir_all(&misplaced).unwrap();
        hmod(&misplaced, "machine_sample");
        let manifest = "id='plain'\nname='Plain'\nversion='1.0.0'\napi='0.2'\nside='both'\ncapabilities=['registries']\n";
        let plain = horizon_mods::Package::write_files(manifest, &[]).unwrap();
        std::fs::write(misplaced.join("plain.hmod"), plain).unwrap();
        let base = f.0.join("resources");
        std::fs::create_dir_all(&base).unwrap();
        // A prepared root's registry report: block entity registration reads it (B1).
        std::fs::write(
            base.join("registries.json"),
            "{\"minecraft:block_entity_type\":{\"entries\":{}}}",
        )
        .unwrap();
        std::env::set_current_dir(&here).unwrap();
        let sources = ModsState {
            native: std::sync::Arc::new(native),
            ..Default::default()
        };
        let enabled = std::collections::BTreeSet::from(["plain".to_owned()]);
        assert_eq!(content_root(&base, &enabled, &sources).unwrap(), base);
        let local = horizon_protocol::mod_list::local();
        let ids: Vec<_> = local.iter().map(|m| m.id.as_str()).collect();
        if mode.as_deref() == Some("compiled") {
            assert_eq!(ids, ["machine_sample", "plain"]);
            assert_eq!(local[0].version, "0.3.0");
        } else {
            assert_eq!(ids, ["plain"]);
        }
        std::env::set_current_dir(&f.0).unwrap();
    }

    #[test]
    fn the_crash_report_lists_the_embedded_native_mods() {
        let built = [
            BUILT[0],
            BuiltMod {
                id: "pipes",
                version: "2.0",
                side: horizon_mods::Side::Client,
            },
        ];
        assert_eq!(
            native_mods_detail(&built).as_deref(),
            Some("machine_sample 0.3.0, pipes 2.0")
        );
        assert_eq!(native_mods_detail(&[]), None);
    }

    #[test]
    fn native_mods_installed_but_not_built_get_a_row_saying_so() {
        let f = Fixture::new("not-built");
        for id in ["veinminer", "zoom", "minimap", "machine_sample"] {
            hmod(&f.0, id);
        }
        std::fs::write(f.0.join("notes.txt"), "not a mod").unwrap();
        std::fs::write(f.0.join("broken.hmod"), "not a zip").unwrap();
        let rows = not_built(&f.0, &[], &[]);
        let ids: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["machine_sample", "minimap", "veinminer", "zoom"]);
        assert_eq!(rows[2].title, "veinminer 0.3.0");
        assert_eq!(rows[2].problem.as_deref(), Some(mod_list::NOT_BUILT));
        assert!(rows.iter().all(|r| r.native.is_none() && !r.dev));
        let listed = Installed {
            id: "minimap".into(),
            ..rows[0].clone()
        };
        let rows = not_built(&f.0, BUILT, &[listed]);
        let ids: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["veinminer", "zoom"]);
        assert!(not_built(&f.0.join("missing"), &[], &[]).is_empty());
    }
}
