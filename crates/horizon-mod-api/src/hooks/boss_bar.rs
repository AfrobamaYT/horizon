//! Upstream capability retained through its existing fork handle.
pub trait Notes {
    #[notes = r###"`native_boss_bar(uuid, id, bar)` (`host.boss_bar`) shows one player a `NativeBossBar` (name, progress, colour, overlay, flags) under a mod's id, changes it, or takes it away (None), as Bukkit's `BossBar.addPlayer`/`removePlayer`; kept by UUID while they are off. Bundles need "hooks": @API:boss_bar@."###]
    fn boss_bar();
}
