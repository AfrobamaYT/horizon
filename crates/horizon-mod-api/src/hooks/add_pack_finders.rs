//! Repository discovery, separately from resource files and reload listeners.
pub trait Hooks {
    /// Add discoverable resource or data packs before a repository selects its
    /// packs (`AddPackFindersEvent`). The builder reports the repository's pack
    /// type. Runs at startup and whenever that repository is rediscovered; no
    /// world state is available during startup registration.
    #[hook(side = "both", contract = "listen", neoforge = "AddPackFindersEvent")]
    fn add_pack_finders();
}
