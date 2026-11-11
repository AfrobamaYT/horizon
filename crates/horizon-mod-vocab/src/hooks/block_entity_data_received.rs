//! Client block entity synchronization notice.
pub trait Hooks {
    /// The synchronized `tag` has been merged into this client's block entity.
    /// Called once for a chunk's initial tag and for each later data packet,
    /// only for a matching mod block entity that the client created.
    /// Reads through `entity` already see the merged data. This notice cannot
    /// change the received tag. Missing tags and vanilla block entities call nothing.
    #[hook(
        side = "client",
        contract = "listen",
        scope = "block_entity",
        key = "entity",
        neoforge = "IBlockEntityExtension.onDataPacket and handleUpdateTag (unverified)"
    )]
    fn block_entity_data_received();
}
