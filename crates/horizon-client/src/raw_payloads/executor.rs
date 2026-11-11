//! NETWORK callbacks have no World borrow. Their replies and main work publish on success.
use super::{Body, Endpoint, Incoming, Prepared, Side};
use mod_api::{
    HandlerThread, IPayloadContext, PayloadAccepted, PayloadBytes, PayloadContextError,
    PayloadDirection, PayloadId, PayloadProtocol, PayloadSendError, PayloadWork, PayloadWorkStatus,
    PlayerRef, Text,
};
use std::sync::Arc;

enum After {
    Reply(Prepared),
    Work(Incoming),
}
const _: () = assert!(std::mem::size_of::<After>() <= 256);

/// Called by the physical receiving executor. Main events use its ordinary ordered stream.
pub fn dispatch(
    mut incoming: Incoming,
    endpoint: Endpoint,
    mut main: impl FnMut(Incoming) -> Result<(), String>,
) {
    if !incoming.epoch.active() {
        return;
    }
    if matches!(incoming.body, Body::Work(_)) || incoming.receiver.thread == HandlerThread::Main {
        if let Err(error) = main(incoming) {
            endpoint.disconnect(failure(&error));
        }
        return;
    }
    let Body::Payload(bytes) =
        std::mem::replace(&mut incoming.body, Body::Payload(PayloadBytes(Vec::new())))
    else {
        unreachable!("work is sent to main above");
    };
    let mut context = NetworkContext {
        incoming: &incoming,
        endpoint: &endpoint,
        after: Vec::new(),
        disconnect: None,
    };
    match incoming.receiver.handle(bytes, &mut context) {
        Err(error) => {
            // Drop reservations and guest work before closing, publishing no partial actions.
            drop(context);
            endpoint.disconnect(failure(&format!(
                "payload {} failed: {error}",
                incoming.id.0
            )));
        }
        Ok(()) => {
            let reason = context.disconnect.take();
            let after = std::mem::take(&mut context.after);
            let result = incoming.epoch.with_active(|| {
                for action in after {
                    match action {
                        After::Reply(reply) => reply.publish().map_err(|error| {
                            format!("payload {} reply refused: {error:?}", incoming.id.0)
                        })?,
                        After::Work(work) => main(work)?,
                    }
                }
                Ok::<(), String>(())
            });
            if let Some(Err(error)) = result {
                endpoint.disconnect(failure(&error));
            } else if let Some(reason) = reason {
                endpoint.disconnect(reason);
            }
        }
    }
}
fn failure(message: &str) -> Text {
    Text::translate("disconnect.genericReason", vec![Text::literal(message)])
}
struct NetworkContext<'a> {
    incoming: &'a Incoming,
    endpoint: &'a Endpoint,
    after: Vec<After>,
    disconnect: Option<Text>,
}
impl IPayloadContext for NetworkContext<'_> {
    fn player(&self) -> Result<PlayerRef, PayloadContextError> {
        if !self.incoming.epoch.active() || self.disconnect.is_some() {
            return Err(PayloadContextError::Disconnected);
        }
        if self.incoming.epoch.phase == horizon_protocol::payload_negotiation::Phase::Configuration
        {
            return Err(PayloadContextError::NoPlayer);
        }
        self.endpoint
            .transport()
            .player()
            .ok_or(PayloadContextError::NoPlayer)
    }
    fn flow(&self) -> PayloadDirection {
        match self.endpoint.side() {
            Side::Client => PayloadDirection::ToClient,
            Side::Server => PayloadDirection::ToServer,
        }
    }
    fn protocol(&self) -> PayloadProtocol {
        match self.incoming.epoch.phase {
            horizon_protocol::payload_negotiation::Phase::Configuration => {
                PayloadProtocol::Configuration
            }
            horizon_protocol::payload_negotiation::Phase::Play => PayloadProtocol::Play,
        }
    }
    fn server_host(&mut self) -> Result<&mut dyn mod_api::ServerHost, PayloadContextError> {
        if !self.incoming.epoch.active() || self.disconnect.is_some() {
            return Err(PayloadContextError::Disconnected);
        }
        if self.endpoint.side() != Side::Server {
            return Err(PayloadContextError::WrongSide);
        }
        Err(PayloadContextError::WrongThread)
    }
    fn client_host(&mut self) -> Result<&mut dyn mod_api::ClientHost, PayloadContextError> {
        if !self.incoming.epoch.active() || self.disconnect.is_some() {
            return Err(PayloadContextError::Disconnected);
        }
        if self.endpoint.side() != Side::Client {
            return Err(PayloadContextError::WrongSide);
        }
        Err(PayloadContextError::WrongThread)
    }
    fn reply(
        &mut self,
        id: &PayloadId,
        bytes: PayloadBytes,
    ) -> Result<PayloadAccepted, PayloadSendError> {
        if self.disconnect.is_some() {
            return Err(PayloadSendError::Disconnected);
        }
        let reply = self.endpoint.prepare(&self.incoming.epoch, id, bytes)?;
        self.after.push(After::Reply(reply));
        Ok(PayloadAccepted::Queued)
    }
    fn disconnect(&mut self, reason: Text) {
        self.disconnect = Some(reason);
    }
    fn enqueue_work(
        &mut self,
        work: Arc<dyn PayloadWork>,
    ) -> Result<PayloadWorkStatus, PayloadContextError> {
        if self.disconnect.is_some() {
            return Err(PayloadContextError::Disconnected);
        }
        let work = self.incoming.work(self.endpoint.transport(), work)?;
        self.after.push(After::Work(work));
        Ok(PayloadWorkStatus::Queued)
    }
}
