//! Owning storage for a resident holder, distinct from detached map snapshots.

use super::{AttachmentAccess, AttachmentError, AttachmentHost, Attachments, Holder};
use std::ops::{Deref, DerefMut};

/// A canonical map whose owning storage retires it on destruction.
///
/// Moves preserve publication. Clones contain detached data only, so saving or
/// journaling a clone never owns the original holder's residency.
#[doc(hidden)]
#[derive(Clone, Debug, Default)]
pub struct ResidentAttachments(Attachments);

impl Drop for ResidentAttachments {
    fn drop(&mut self) {
        self.0.unbind_index();
    }
}

impl Deref for ResidentAttachments {
    type Target = Attachments;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ResidentAttachments {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AttachmentHost for ResidentAttachments {
    fn attachments(
        &mut self,
        _: Holder,
        _: AttachmentAccess,
    ) -> Result<&mut Attachments, AttachmentError> {
        Ok(&mut self.0)
    }
}
