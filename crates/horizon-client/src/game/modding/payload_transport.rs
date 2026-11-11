//! Sole-owner raw payload execution and root-commit publication.
use super::Game;
use horizon_client::{
    lan::PlayerSender,
    raw_payloads::{self, Incoming, Side, connection::Epoch},
};
use horizon_protocol::payload_negotiation::Phase;
use mod_api::{
    HandlerThread, IPayloadContext, PayloadAccepted, PayloadBytes, PayloadContextError,
    PayloadDirection, PayloadId, PayloadProtocol, PayloadSendError, PayloadWork, PayloadWorkStatus,
    PlayerRef, Text,
};
use std::sync::Arc;

enum Prepared {
    Client(horizon_client::net::PreparedPayload),
    Server(horizon_client::lan::conn::PreparedPayload),
}
/// Fits the 512-byte journal-growth allowance charged by each prepare operation.
pub(super) struct Deferred {
    epoch: Arc<Epoch>,
    prepared: Prepared,
}
const _: () = assert!(std::mem::size_of::<Deferred>() <= 256);
impl Deferred {
    pub(super) fn publish(self) {
        if !self.epoch.active() {
            return;
        }
        let _ = match self.prepared {
            Prepared::Client(send) => send.publish(),
            Prepared::Server(send) => send.publish(),
        };
    }
}
#[derive(Clone)]
enum Target {
    Client(u64),
    Server(PlayerSender),
}

impl Game<'_> {
    pub(super) fn player_send_payload(
        &mut self,
        player: PlayerRef,
        id: &PayloadId,
        bytes: PayloadBytes,
    ) -> Result<PayloadAccepted, PayloadSendError> {
        let sender = self
            .online_player_record(player)
            .ok_or(PayloadSendError::StaleEpoch)?
            .sender
            .clone();
        let epoch = sender
            .payload_transport()
            .epoch()
            .ok_or(PayloadSendError::NotNegotiated)?;
        self.queue_payload(Target::Server(sender), epoch, id, bytes)
    }
    pub(super) fn send_payload_to_server(
        &mut self,
        id: &PayloadId,
        bytes: PayloadBytes,
    ) -> Result<PayloadAccepted, PayloadSendError> {
        let remote = self.remote.as_ref().ok_or(PayloadSendError::Disconnected)?;
        let epoch = remote
            .session
            .payload_transport()
            .epoch()
            .ok_or(PayloadSendError::NotNegotiated)?;
        let target = Target::Client(remote.session.serial());
        self.queue_payload(target, epoch, id, bytes)
    }
    fn queue_payload(
        &mut self,
        target: Target,
        epoch: Arc<Epoch>,
        id: &PayloadId,
        bytes: PayloadBytes,
    ) -> Result<PayloadAccepted, PayloadSendError> {
        if !epoch.active() {
            return Err(PayloadSendError::StaleEpoch);
        }
        epoch.peer.can_send(epoch.phase, id)?;
        let prepared = match target {
            Target::Client(serial) => {
                let remote = self
                    .remote
                    .as_ref()
                    .filter(|remote| remote.session.serial() == serial)
                    .ok_or(PayloadSendError::StaleEpoch)?;
                let body = raw_payloads::encode(epoch.phase, Side::Client, id, bytes);
                Prepared::Client(remote.session.prepare_payload(epoch.phase, body, id)?)
            }
            Target::Server(sender) => {
                let body = raw_payloads::encode(epoch.phase, Side::Server, id, bytes);
                Prepared::Server(sender.prepare_payload(body, id)?)
            }
        };
        self.after_commit(super::transactions::After::Payload(Deferred {
            epoch,
            prepared,
        }));
        Ok(PayloadAccepted::Queued)
    }
    /// The window's ordinary ordered ingress invokes this on its main executor.
    pub(super) fn receive_client_payload(
        &mut self,
        incoming: Incoming,
    ) -> horizon_client::Result<()> {
        let Some(remote) = self.remote.as_ref() else {
            return Ok(());
        };
        let target = Target::Client(remote.session.serial());
        let player = (incoming.epoch.phase == Phase::Play)
            .then(|| self.player(remote.profile))
            .flatten();
        self.run_payload(target, player, incoming)
    }
    /// No implicit guest state swap: identity and player handle come from the connection.
    pub(super) fn receive_guest_payload(
        &mut self,
        conn: u32,
        packet: horizon_protocol::packets::custom_payload::CustomPayload,
    ) {
        let Some(guest) = self.lan.as_ref().and_then(|host| host.hub.guest(conn)) else {
            return;
        };
        let sender = guest.sender.clone();
        let player = self.player(guest.uuid);
        let incoming = Incoming::new(sender.payload_transport(), Phase::Play, packet);
        match incoming {
            Ok(incoming) => {
                let _ = self.run_payload(Target::Server(sender), player, incoming);
            }
            Err(error) => sender.disconnect(horizon_client::lan::data::component_nbt(
                &horizon_client::text::Text::tr(
                    "disconnect.genericReason",
                    &[&format!("payload refused: {error:?}")],
                )
                .component(),
            )),
        }
    }
    fn run_payload(
        &mut self,
        target: Target,
        player: Option<PlayerRef>,
        incoming: Incoming,
    ) -> horizon_client::Result<()> {
        let Incoming {
            epoch,
            id,
            bytes,
            receiver,
            main,
        } = incoming;
        if !epoch.active() {
            return Ok(());
        }
        let seal = self.transaction_open();
        let mut context = MainContext {
            game: self,
            epoch: &epoch,
            target: target.clone(),
            player,
            receiver: &receiver,
            disconnect: None,
            failure: None,
        };
        let result = if receiver.thread == HandlerThread::Main {
            receiver.handle(bytes, &mut context)
        } else {
            Err("payload was not dispatched on its declared network executor".into())
        };
        let reason = context.disconnect.take();
        let failure = context.failure.take().or_else(|| result.err());
        context.game.transaction_close(&seal, failure.is_none());
        if let Some(error) = failure {
            let reason = Text::translate(
                "disconnect.genericReason",
                vec![Text::literal(format!("payload {} failed: {error}", id.0))],
            );
            context.game.payload_disconnect(target, reason)?;
        } else if let Some(reason) = reason {
            context.game.payload_disconnect(target, reason)?;
        }
        drop(main);
        Ok(())
    }
    fn payload_disconnect(&mut self, target: Target, reason: Text) -> horizon_client::Result<()> {
        use super::api_values::TryIntoEngine;
        let reason: horizon_client::text::Text = reason.try_into_engine(&())?;
        match target {
            Target::Client(serial)
                if self
                    .remote
                    .as_ref()
                    .is_some_and(|remote| remote.session.serial() == serial) =>
            {
                self.leave_server(Some(reason.plain()))?
            }
            Target::Client(_) => {}
            Target::Server(sender) => sender.disconnect(horizon_client::lan::data::component_nbt(
                &reason.component(),
            )),
        }
        Ok(())
    }
}

struct MainContext<'a, 'resources> {
    game: &'a mut Game<'resources>,
    epoch: &'a Arc<Epoch>,
    target: Target,
    player: Option<PlayerRef>,
    receiver: &'a mod_api::__private::game::PayloadReceiver,
    disconnect: Option<Text>,
    failure: Option<String>,
}
impl IPayloadContext for MainContext<'_, '_> {
    fn player(&self) -> Result<PlayerRef, PayloadContextError> {
        if !self.epoch.active() || self.disconnect.is_some() {
            return Err(PayloadContextError::Disconnected);
        }
        self.player.ok_or(PayloadContextError::NoPlayer)
    }
    fn flow(&self) -> PayloadDirection {
        match self.target {
            Target::Client(_) => PayloadDirection::ToClient,
            Target::Server(_) => PayloadDirection::ToServer,
        }
    }
    fn protocol(&self) -> PayloadProtocol {
        match self.epoch.phase {
            Phase::Configuration => PayloadProtocol::Configuration,
            Phase::Play => PayloadProtocol::Play,
        }
    }
    fn server_host(&mut self) -> Result<&mut dyn mod_api::ServerHost, PayloadContextError> {
        if !self.epoch.active() || self.disconnect.is_some() {
            return Err(PayloadContextError::Disconnected);
        }
        if !matches!(self.target, Target::Server(_)) {
            return Err(PayloadContextError::WrongSide);
        }
        Ok(self.game)
    }
    fn client_host(&mut self) -> Result<&mut dyn mod_api::ClientHost, PayloadContextError> {
        if !self.epoch.active() || self.disconnect.is_some() {
            return Err(PayloadContextError::Disconnected);
        }
        if !matches!(self.target, Target::Client(_)) {
            return Err(PayloadContextError::WrongSide);
        }
        Ok(self.game)
    }
    fn reply(
        &mut self,
        id: &PayloadId,
        bytes: PayloadBytes,
    ) -> Result<PayloadAccepted, PayloadSendError> {
        if self.disconnect.is_some() {
            return Err(PayloadSendError::Disconnected);
        }
        self.game
            .queue_payload(self.target.clone(), self.epoch.clone(), id, bytes)
    }
    fn disconnect(&mut self, reason: Text) {
        self.disconnect = Some(reason);
    }
    fn enqueue_work(
        &mut self,
        work: Arc<dyn PayloadWork>,
    ) -> Result<PayloadWorkStatus, PayloadContextError> {
        if !self.epoch.active() || self.disconnect.is_some() {
            return Err(PayloadContextError::Disconnected);
        }
        let receiver = self.receiver.clone();
        if let Err(error) = receiver.run_work(work.as_ref(), self) {
            self.failure = Some(error);
            return Err(PayloadContextError::Disconnected);
        }
        Ok(PayloadWorkStatus::Complete)
    }
}

#[cfg(test)]
#[path = "payload_transport/tests.rs"]
mod tests;
