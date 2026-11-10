//! Repository-selected identity. An upstream build retains its original namespaces.
macro_rules! identity {
    ($($name:ident: $upstream:literal => $fork:literal),* $(,)?) => {$ (
        pub const $name: &str = if cfg!(horizon_fork_launcher) { $fork } else { $upstream };
    )*};
}
identity! {
    VERSION_PREFIX: "horizon-launcher " => "horizon-fork-launcher ",
    DESKTOP_ENTRY: "horizon-launcher.desktop" => "horizon-fork-launcher.desktop",
    QUIT_LABEL: "Quit Horizon Launcher" => "Quit Horizon Fork Launcher (arismoko)",
    DISPLAY_NAME: "Horizon Launcher" => "Horizon Fork Launcher (arismoko)",
    EXECUTABLE: "horizon-launcher" => "horizon-fork-launcher",
    WINDOWS_EXECUTABLE: "horizon-launcher.exe" => "horizon-fork-launcher.exe",
    WINDOWS_EXECUTABLE_PREFIX: "horizon-launcher.exe." => "horizon-fork-launcher.exe.",
    APP_ID: "horizon-launcher" => "io.github.arismoko.horizonfork.launcher",
    KEYCHAIN_SERVICE: "dev.playhorizon.launcher" => "io.github.arismoko.horizonfork.launcher",
    LOCATION_FOLDER: "horizon-launcher-location" => "horizon-fork-arismoko-location",
    LAUNCHER_FOLDER: "horizon-launcher" => "horizon-fork-arismoko/launcher",
    GAME_FOLDER: "horizon" => "horizon-fork-arismoko/game",
    PUBLISHER: "Horizon" => "arismoko",
    ABOUT_URL: "https://playhorizon.dev" => "https://github.com/arismoko/horizon",
    UNINSTALL_KEY: "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\HorizonLauncher" => "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ArismokoHorizonForkLauncher",
    INSTALL_KEY: "Software\\Horizon\\Launcher" => "Software\\ArismokoHorizonFork\\Launcher",
    REGISTRY_PARENT: "Software\\Horizon" => "Software\\ArismokoHorizonFork",
    UNINSTALL_PREFIX: "horizon-launcher-uninstall-" => "horizon-fork-launcher-uninstall-",
    SETUP_PREFIX: "horizon-launcher-setup" => "horizon-fork-launcher-setup",
    PROCESS_FILTER: "IMAGENAME eq horizon-launcher.exe" => "IMAGENAME eq horizon-fork-launcher.exe",
}
