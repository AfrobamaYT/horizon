//! One bounded writer owner. Receiving callbacks never block their own receive loop
//! while a large reply waits for the peer to drain its socket or memory pipe.
use horizon_mod_runtime::byte_channel::{self, Permit, ReserveError, Sender, TrySendError};
use horizon_protocol::{
    Error, Result,
    adapter::ServerVersion,
    connection::{Closer, ConnWriter, Framing},
    frame::FrameSize,
};
use mod_api::{PayloadFrameStage, PayloadId, PayloadSendError};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

const CAPACITY: usize = 16 * 1024 * 1024;
// Covers the pending transaction journal record as well as the queue's own node.
const PUBLICATION_OVERHEAD: usize = 512;
enum WriteJob {
    Packet(Vec<u8>),
    Contents(Vec<u8>),
}

pub(super) struct QueuedWriter {
    sender: Sender<WriteJob>,
    framing: Framing,
    closer: Arc<Closer>,
    alive: Arc<AtomicBool>,
    version: ServerVersion,
    secure_chat: Box<dyn Fn() -> Option<bool> + Send + Sync>,
}

/// Prepared once, charged immediately, invisible to the writer until root commit.
pub struct PreparedPayload {
    permit: Permit<WriteJob>,
    contents: Vec<u8>,
    alive: Arc<AtomicBool>,
}
impl PreparedPayload {
    pub fn publish(self) -> std::result::Result<(), PayloadSendError> {
        if !self.alive.load(Ordering::Acquire) {
            return Err(PayloadSendError::Disconnected);
        }
        self.permit
            .publish(WriteJob::Contents(self.contents))
            .map_err(|_| PayloadSendError::Disconnected)
    }
}
impl QueuedWriter {
    pub(super) fn new(mut writer: ConnWriter) -> Result<Self> {
        let framing = writer.framing();
        let closer = Arc::new(writer.closer()?);
        let alive = Arc::new(AtomicBool::new(true));
        let version = writer.server_version();
        let secure_chat = Box::new(writer.legacy_secure_chat_observer());
        let (sender, receiver) = byte_channel::channel(CAPACITY).expect("nonzero writer capacity");
        let running = alive.clone();
        std::thread::Builder::new()
            .name("horizon-net-write".into())
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    let result = match job {
                        WriteJob::Packet(body) => writer.send(&body),
                        WriteJob::Contents(contents) => writer.send_contents(&contents),
                    };
                    if result.is_err() {
                        break;
                    }
                }
                running.store(false, Ordering::Release);
                writer.shutdown();
            })
            .map_err(|error| Error::new(format!("cannot start client writer: {error}")))?;
        Ok(Self {
            sender,
            framing,
            closer,
            alive,
            version,
            secure_chat,
        })
    }
    pub(super) fn server_version(&self) -> ServerVersion {
        self.version
    }
    pub(super) fn legacy_secure_chat_enforcement(&self) -> Option<bool> {
        (self.secure_chat)()
    }
    pub(super) fn shutdown(&self) {
        self.alive.store(false, Ordering::Release);
        self.closer.close();
    }
    pub(super) fn send(&self, body: &[u8]) -> Result<()> {
        if !self.alive.load(Ordering::Acquire) {
            return Err(Error::new("client writer disconnected"));
        }
        let owned = body.to_vec();
        let capacity = owned.capacity();
        match self.sender.try_send(WriteJob::Packet(owned), capacity) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                self.shutdown();
                Err(Error::new("client writer backpressure"))
            }
            Err(TrySendError::Disconnected(_)) => Err(Error::new("client writer disconnected")),
        }
    }
    pub(super) fn prepare_payload(
        &self,
        body: Vec<u8>,
        id: &PayloadId,
    ) -> std::result::Result<PreparedPayload, PayloadSendError> {
        if !self.alive.load(Ordering::Acquire) {
            return Err(PayloadSendError::Disconnected);
        }
        let contents = self.framing.contents_owned(body).map_err(|error| {
            if let Some(detail) = error.frame_limit() {
                PayloadSendError::FrameTooLarge {
                    id: id.clone(),
                    stage: match detail.stage {
                        FrameSize::Encoded => PayloadFrameStage::Encoded,
                        FrameSize::Uncompressed => PayloadFrameStage::Uncompressed,
                    },
                    size: detail.size as u64,
                    limit: detail.limit as u64,
                }
            } else {
                PayloadSendError::EncodeFailed(error.to_string())
            }
        })?;
        let permit = self
            .sender
            .reserve(contents.capacity() + PUBLICATION_OVERHEAD)
            .map_err(|error| match error {
                ReserveError::Full => PayloadSendError::Backpressure,
                ReserveError::Disconnected => PayloadSendError::Disconnected,
            })?;
        Ok(PreparedPayload {
            permit,
            contents,
            alive: self.alive.clone(),
        })
    }
}
#[cfg(test)]
mod tests;

impl Drop for QueuedWriter {
    fn drop(&mut self) {
        self.shutdown();
    }
}
