//! Frozen raw-payload declarations and per-connection dispatch tables.
//! Application packets retain their declared vanilla custom-payload channel.
use horizon_protocol::{
    packets::custom_payload::{PayloadCodecs, PayloadFlow, PayloadProtocol as CodecProtocol},
    payload_negotiation::{self as negotiation, Flow, Manifest, Phase, RegistrationPhase},
};
use mod_api::__private::game::{PayloadReceiver, Payloads, installed_payloads};
use mod_api::{PayloadDirection, PayloadId, PayloadProtocol, PayloadSendError};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

/// Logical receiving side, never inferred from application bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Client,
    Server,
}
impl Side {
    pub fn incoming(self) -> PayloadDirection {
        match self {
            Self::Client => PayloadDirection::ToClient,
            Self::Server => PayloadDirection::ToServer,
        }
    }
    fn outgoing(self) -> Flow {
        match self {
            Self::Client => Flow::ToServer,
            Self::Server => Flow::ToClient,
        }
    }
}

/// One installation's immutable declaration snapshot, shared by its connections.
pub struct Local {
    pub manifest: Manifest,
    pub codecs: PayloadCodecs,
    registry: Option<Arc<Payloads>>,
    declarations: [HashMap<String, Flow>; 2],
}
impl Local {
    /// Capture only enabled declarations. Client setup refuses missing client callbacks;
    /// dedicated servers do not require callbacks that they cannot receive.
    pub fn capture(side: Side) -> Result<Arc<Self>, String> {
        let Some(registry) = installed_payloads() else {
            return Ok(Self::empty());
        };
        if side == Side::Client {
            registry
                .validate_client_handlers()
                .map_err(|errors| errors.join("; "))?;
        }
        let mut manifest = Manifest::builder();
        for declaration in registry.declarations() {
            if declaration.id.0 == negotiation::CHANNEL {
                return Err(format!(
                    "payload {} is reserved for transport metadata",
                    declaration.id.0
                ));
            }
            let phase = match declaration.protocol {
                PayloadProtocol::Configuration => RegistrationPhase::Configuration,
                PayloadProtocol::Play => RegistrationPhase::Play,
                PayloadProtocol::Common => RegistrationPhase::Common,
            };
            let flow = match declaration.direction {
                PayloadDirection::ToClient => Flow::ToClient,
                PayloadDirection::ToServer => Flow::ToServer,
                PayloadDirection::Bidirectional => Flow::Bidirectional,
            };
            manifest
                .register(
                    phase,
                    &declaration.id.0,
                    flow,
                    &declaration.version,
                    declaration.optional,
                )
                .map_err(|error| error.to_string())?;
        }
        let manifest = manifest.finish().map_err(|error| error.to_string())?;
        if manifest.entries().is_empty() {
            return Ok(Self::empty());
        }
        Ok(Arc::new(Self::new(manifest, Some(registry))))
    }
    fn empty() -> Arc<Self> {
        static EMPTY: OnceLock<Arc<Local>> = OnceLock::new();
        EMPTY
            .get_or_init(|| Arc::new(Self::new(Manifest::default(), None)))
            .clone()
    }
    fn new(manifest: Manifest, registry: Option<Arc<Payloads>>) -> Self {
        // The metadata channel must also be accepted by an otherwise unmodded endpoint:
        // an optional peer's offer can exceed the unknown serverbound discard limit.
        let mut channels = vec![
            (
                negotiation::CHANNEL.to_owned(),
                CodecProtocol::Configuration,
                PayloadFlow::ToClient,
            ),
            (
                negotiation::CHANNEL.to_owned(),
                CodecProtocol::Configuration,
                PayloadFlow::ToServer,
            ),
        ];
        let mut declarations = [HashMap::new(), HashMap::new()];
        for entry in manifest.entries() {
            let protocol = match entry.phase() {
                Phase::Configuration => CodecProtocol::Configuration,
                Phase::Play => CodecProtocol::Play,
            };
            declarations[index(entry.phase())].insert(entry.id().to_owned(), entry.flow());
            for flow in [PayloadFlow::ToClient, PayloadFlow::ToServer] {
                if entry.flow() == Flow::Bidirectional
                    || matches!(
                        (entry.flow(), flow),
                        (Flow::ToClient, PayloadFlow::ToClient)
                            | (Flow::ToServer, PayloadFlow::ToServer)
                    )
                {
                    channels.push((entry.id().to_owned(), protocol, flow));
                }
            }
        }
        Self {
            manifest,
            codecs: PayloadCodecs::from_channels(channels),
            registry,
            declarations,
        }
    }
    /// Whether a channel belongs to this transport, including wrong-phase declarations.
    pub fn owns(&self, id: &str) -> bool {
        id == negotiation::CHANNEL || self.declarations.iter().any(|phase| phase.contains_key(id))
    }
    /// Freeze sole-owner callbacks once, outside packet dispatch.
    pub fn freeze(
        self: &Arc<Self>,
        accepted: &[negotiation::Entry],
        side: Side,
    ) -> Result<Peer, String> {
        let mut phases = [HashMap::new(), HashMap::new()];
        for entry in accepted {
            let receives = entry.flow() == Flow::Bidirectional
                || matches!(
                    (entry.flow(), side),
                    (Flow::ToClient, Side::Client) | (Flow::ToServer, Side::Server)
                );
            let receiver = if receives {
                let protocol = match entry.phase() {
                    Phase::Configuration => PayloadProtocol::Configuration,
                    Phase::Play => PayloadProtocol::Play,
                };
                Some(
                    self.registry
                        .as_ref()
                        .and_then(|registry| {
                            registry.handler(entry.id(), protocol, side.incoming())
                        })
                        .ok_or_else(|| {
                            format!(
                                "payload {} ({:?}) has no enabled receiving handler",
                                entry.id(),
                                entry.phase()
                            )
                        })?,
                )
            } else {
                None
            };
            phases[index(entry.phase())].insert(entry.id().to_owned(), receiver);
        }
        Ok(Peer {
            local: self.clone(),
            side,
            phases,
        })
    }
}

/// Accepted channel table for one connection epoch. No registration scan on dispatch.
pub struct Peer {
    pub local: Arc<Local>,
    side: Side,
    phases: [HashMap<String, Option<PayloadReceiver>>; 2],
}
impl Peer {
    /// One lookup for an active receiving packet. An absent callback is a protocol refusal,
    /// never a silent skip. Callers separately distinguish unrelated engine/plugin channels.
    pub fn receiver(&self, phase: Phase, id: &str) -> Result<&PayloadReceiver, PayloadSendError> {
        match self.phases[index(phase)].get(id) {
            Some(Some(receiver)) if receiver.enabled() => Ok(receiver),
            Some(Some(_)) => Err(PayloadSendError::Disconnected),
            Some(None) => Err(PayloadSendError::WrongDirection(PayloadId(id.to_owned()))),
            None => Err(self.missing(phase, id)),
        }
    }
    /// Resolve local authority, phase, direction and peer support before preparing bytes.
    pub fn can_send(&self, phase: Phase, id: &PayloadId) -> Result<(), PayloadSendError> {
        let flow = self.local.declarations[index(phase)]
            .get(&id.0)
            .ok_or_else(|| self.missing(phase, &id.0))?;
        if *flow != Flow::Bidirectional && *flow != self.side.outgoing() {
            return Err(PayloadSendError::WrongDirection(id.clone()));
        }
        if !self.phases[index(phase)].contains_key(&id.0) {
            return Err(PayloadSendError::UnsupportedPeer(id.clone()));
        }
        Ok(())
    }
    fn missing(&self, phase: Phase, id: &str) -> PayloadSendError {
        let id = PayloadId(id.to_owned());
        if self.local.declarations[index(phase)].contains_key(&id.0) {
            PayloadSendError::UnsupportedPeer(id)
        } else if self.local.owns(&id.0) {
            PayloadSendError::WrongProtocol(id)
        } else {
            PayloadSendError::UnknownPayload(id)
        }
    }
}
fn index(phase: Phase) -> usize {
    match phase {
        Phase::Configuration => 0,
        Phase::Play => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn installation(flow: Flow) -> Arc<Local> {
        let mut builder = Manifest::builder();
        builder
            .register(RegistrationPhase::Play, "test:raw", flow, "1", true)
            .unwrap();
        Arc::new(Local::new(builder.finish().unwrap(), None))
    }
    #[test]
    fn send_authority_requires_local_phase_direction_and_peer_intersection() {
        let local = installation(Flow::ToServer);
        let id = PayloadId("test:raw".into());
        let absent = local.freeze(&[], Side::Client).unwrap();
        assert_eq!(
            absent.can_send(Phase::Play, &id),
            Err(PayloadSendError::UnsupportedPeer(id.clone()))
        );
        let peer = local
            .freeze(local.manifest.entries(), Side::Client)
            .unwrap();
        assert_eq!(peer.can_send(Phase::Play, &id), Ok(()));
        assert_eq!(
            peer.can_send(Phase::Configuration, &id),
            Err(PayloadSendError::WrongProtocol(id.clone()))
        );
        let foreign = PayloadId("other:raw".into());
        assert_eq!(
            peer.can_send(Phase::Play, &foreign),
            Err(PayloadSendError::UnknownPayload(foreign))
        );
        let wrong = installation(Flow::ToClient)
            .freeze(&[], Side::Client)
            .unwrap();
        assert_eq!(
            wrong.can_send(Phase::Play, &id),
            Err(PayloadSendError::WrongDirection(id))
        );
    }
    #[test]
    fn accepted_incoming_channel_without_receiver_refuses_before_dispatch() {
        let local = installation(Flow::ToServer);
        assert!(
            local
                .freeze(local.manifest.entries(), Side::Server)
                .is_err()
        );
    }
}
