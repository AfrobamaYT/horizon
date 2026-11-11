//! An owned raw message queued in the same ingress stream as ordinary packets.
use super::{
    Transport,
    connection::{Epoch, MainTask},
};
use horizon_protocol::{packets::custom_payload::CustomPayload, payload_negotiation::Phase};
use mod_api::{__private::game::PayloadReceiver, PayloadBytes, PayloadId, PayloadSendError};
use std::sync::Arc;

pub struct Incoming {
    pub epoch: Arc<Epoch>,
    pub id: PayloadId,
    pub bytes: PayloadBytes,
    pub receiver: PayloadReceiver,
    /// Held through the root transaction, not just until dequeue.
    pub main: MainTask,
}
impl Incoming {
    pub fn new(
        transport: &Transport,
        phase: Phase,
        packet: CustomPayload,
    ) -> Result<Self, PayloadSendError> {
        let epoch = transport
            .epoch()
            .ok_or_else(|| PayloadSendError::UnsupportedPeer(PayloadId(packet.channel.clone())))?;
        if !epoch.active() {
            return Err(PayloadSendError::StaleEpoch);
        }
        if epoch.phase != phase {
            return Err(PayloadSendError::WrongProtocol(PayloadId(packet.channel)));
        }
        let receiver = epoch.peer.receiver(phase, &packet.channel)?.clone();
        let main = epoch.main_task();
        Ok(Self {
            epoch,
            id: PayloadId(packet.channel),
            bytes: PayloadBytes(packet.data),
            receiver,
            main,
        })
    }
}
