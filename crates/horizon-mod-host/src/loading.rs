//! Installed native and wasm code loading.
use horizon_mod_package::selection::LaunchArguments;
pub fn load(mods: Vec<mod_api::ModEntry>, built: &'static [horizon_mod_package::runtime::BuiltMod], launch: &LaunchArguments) -> (horizon_mod_package::runtime::Runtime, Vec<&'static str>) {
    let exe = std::env::current_exe();
    let native: Result<_, Box<dyn std::error::Error + Send + Sync>> = exe
        .as_ref()
        .map_err(|e| e.to_string().into())
        .and_then(|exe| {
            horizon_mod_package::runtime::Runtime::load(exe, built).map_err(Into::into)
        });
    let native = match native {
        Ok(native) => native,
        Err(error) => {
            eprintln!("Horizon: {error}");
            std::process::exit(1);
        }
    };
    let wasm = horizon_mod_wasm::load_installed(&launch.context.game_dir, &launch.context.dev_mods);
    let mut mods = mods;
    mods.extend(wasm.mods);
    // One installed dispatch order for both targets; ties retain bundle order.
    let manifests: Vec<_> = native
        .packages()
        .iter()
        .map(|p| &p.manifest)
        .chain(wasm.manifests.iter())
        .filter(|m| mods.iter().any(|entry| entry.fields().id == m.id))
        .collect();
    assert_eq!(
        manifests.len(),
        mods.len(),
        "loaded code has checked manifests"
    );
    let declared: Vec<_> = manifests
        .iter()
        .map(|m| horizon_mod_package::dependencies::from_manifest(&m.dependencies))
        .collect();
    let inputs: Vec<_> = manifests
        .iter()
        .zip(&declared)
        .map(|(m, dependencies)| horizon_mod_package::dependencies::Mod {
            id: &m.id,
            version: &m.version,
            has_api: true,
            dependencies,
        })
        .collect();
    let order = horizon_mod_package::dependencies::resolve(&inputs)
        .unwrap_or_else(|error| panic!("mod load refused: {error}"));
    let mut entries: std::collections::BTreeMap<_, _> = mods
        .into_iter()
        .map(|entry| (entry.fields().id, entry))
        .collect();
    let mods: Vec<_> = order
        .order
        .into_iter()
        .map(|i| entries.remove(manifests[i].id.as_str()).unwrap())
        .collect();
    let ids: Vec<_> = mods.iter().map(|entry| entry.fields().id).collect();
    mod_api::__private::install(mods.into());
    (native, ids)
}
