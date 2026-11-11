impl BlockEntityRef {
    fn kind(self, _: &dyn ClientHost) -> Option<BlockEntityTypeKey> {
        Some(BlockEntityTypeKey(0))
    }
}
