//! The `default_components_bound` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Items' default components are bound and readable (NeoForge's
    /// `DefaultDataComponentsBoundEvent`); `cause` says how, as `tags_updated`'s does. The
    /// place to build or rebuild caches derived from item defaults (stack sizes, food,
    /// tool rules): from now on the default-component reads return the published
    /// defaults, `modify_default_components`' edits included.
    ///
    /// `ServerDataLoad`: on the server once its data tables are built, on a world's first
    /// open and after `/reload`. `ClientPacketReceived`: on a client once it applied the
    /// tags its server sent, at login and for each update in play. Each fires right before
    /// `tags_updated` with the same cause; `cause.should_update_static_data()` is false
    /// for an integrated server's client, so data the two threads share is updated once.
    ///
    /// Runs in Reloading: published asset/data queries can rebind caches; world handles
    /// and transactions remain unavailable.
    #[hook(
        side = "both",
        contract = "listen",
        neoforge = "DefaultDataComponentsBoundEvent"
    )]
    fn default_components_bound();
}
