/// Read-only block metadata and typed operation errors.
pub mod block_metadata;
pub use block_metadata::BlockStateError;
#[doc(hidden)]
pub use block_metadata::{BlockMetadata, BlockStateRecord};
