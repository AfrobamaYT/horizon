/// Which installations must carry a package, not the executing host's side.
// no wire: the manual Wire codec owns the explicit byte layout and checked reconstruction, rather than a structural derive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Side {
    /// Local content, not compared with the server's mods when joining.
    #[default]
    Client,
    /// Both installations; the server tells clients which version it expects.
    Both,
}
