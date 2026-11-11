//! Typed mod payloads (arismoko/horizon#813), stage 1: the transport's
//! capacities, the direction-typed tokens and the object-safe send services.
//! Nothing registers a payload yet, so a mod cannot hold a token; the codec,
//! registration, negotiation and delivery are the later stages. See
//! `docs/modding/src/concepts/typed-payloads.md`.
use std::marker::PhantomData;

/// The most bytes of a complete typed envelope a client sends a server, in
/// configuration and in play: vanilla 26.3's
/// `ServerboundCustomPayloadPacket.MAX_PAYLOAD_SIZE`.
pub const SERVERBOUND_PAYLOAD_CAPACITY: usize = 32_767;
/// The most bytes of a complete typed envelope a server sends a client, in
/// configuration and in play: vanilla 26.3's
/// `ClientboundCustomPayloadPacket.MAX_PAYLOAD_SIZE`.
pub const CLIENTBOUND_PAYLOAD_CAPACITY: usize = 1_048_576;

/// Which way a payload travels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, horizon_mod_api_macros::Wire)]
// exhaustive: a connection has two ends
pub enum PayloadDirection {
    /// From a client to its server.
    ToServer,
    /// From a server to one client.
    ToClient,
}

impl PayloadDirection {
    /// The most bytes a complete envelope may hold this way, its nested
    /// values and the envelope's own overhead included.
    pub const fn capacity(self) -> usize {
        match self {
            Self::ToServer => SERVERBOUND_PAYLOAD_CAPACITY,
            Self::ToClient => CLIENTBOUND_PAYLOAD_CAPACITY,
        }
    }

    /// Whether a complete envelope of `size` bytes fits this way. The same
    /// answer in configuration and in play; nothing is sent on a refusal.
    pub const fn admit_envelope(self, size: usize) -> Result<(), PayloadSendError> {
        let limit = self.capacity();
        if size > limit {
            return Err(PayloadSendError::PayloadTooLarge {
                size: size as u64,
                limit: limit as u64,
            });
        }
        Ok(())
    }
}

/// The protocol phase a connection is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, horizon_mod_api_macros::Wire)]
// exhaustive: the two phases that carry custom payloads
pub enum ConnectionPhase {
    /// Before the player is admitted to a world.
    Configuration,
    /// The player is in a world.
    Play,
}

/// The phases a payload is declared for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, horizon_mod_api_macros::Wire)]
// exhaustive: one phase or both
pub enum PayloadPhase {
    /// Configuration only.
    Configuration,
    /// Play only.
    Play,
    /// Both phases.
    Common,
}

impl PayloadPhase {
    /// Whether a payload declared for these phases may travel in `phase`.
    pub const fn permits(self, phase: ConnectionPhase) -> bool {
        matches!(
            (self, phase),
            (Self::Common, _)
                | (Self::Configuration, ConnectionPhase::Configuration)
                | (Self::Play, ConnectionPhase::Play)
        )
    }
}

/// Whether a peer must know a payload to be admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, horizon_mod_api_macros::Wire)]
// exhaustive: a peer is admitted without the payload or it is not
pub enum PeerPolicy {
    /// A peer without the payload, or with another schema version of it, is
    /// refused admission.
    Required,
    /// A peer without it joins; a send to that peer is
    /// [`PayloadSendError::UnsupportedPeer`] and no packet.
    Optional,
}

/// Why a typed payload was not accepted for sending. Nothing was sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, horizon_mod_api_macros::Wire)]
#[non_exhaustive]
pub enum PayloadSendError {
    /// The complete envelope is past its direction's capacity.
    PayloadTooLarge {
        /// Bytes of the complete envelope.
        size: u64,
        /// The direction's capacity.
        limit: u64,
    },
    /// The peer did not negotiate this optional payload.
    UnsupportedPeer,
    /// This tick's message or byte budget for the peer is used up.
    Backpressure,
    /// The token is not of this load session: forged, or kept from an
    /// earlier one.
    StaleToken,
    /// The token's payload travels the other way.
    WrongDirection,
    /// The payload is not declared for the connection's phase.
    WrongPhase {
        /// The phase the connection is in.
        phase: ConnectionPhase,
    },
    /// There is no connection to send on.
    NotConnected,
}

/// A registered payload direction with its value type erased: what the
/// object-safe send services take. The game issues one when registration
/// freezes; it names the declaration's slot, its phases, its direction and
/// the load session that issued it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, horizon_mod_api_macros::Wire)]
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct PayloadToken {
    slot: u32,
    load_session: u64,
    phase: PayloadPhase,
    direction: PayloadDirection,
}

impl PayloadToken {
    /// Issues the token of the declaration in `slot`.
    // hidden: a game-side helper, not part of the mod surface
    #[doc(hidden)]
    pub const fn issue(
        slot: u32,
        load_session: u64,
        phase: PayloadPhase,
        direction: PayloadDirection,
    ) -> Self {
        Self {
            slot,
            load_session,
            phase,
            direction,
        }
    }

    /// The phases the payload is declared for.
    pub const fn phase(self) -> PayloadPhase {
        self.phase
    }

    /// The way the payload travels.
    pub const fn direction(self) -> PayloadDirection {
        self.direction
    }

    /// The declaration's slot, when this token may send `direction`-ward on
    /// a connection in `phase` of the load session `load_session`. A send
    /// service checks this before anything else, so a forged or stale token
    /// reaches no codec and no handler.
    pub const fn admit(
        self,
        direction: PayloadDirection,
        load_session: u64,
        phase: ConnectionPhase,
    ) -> Result<u32, PayloadSendError> {
        if self.load_session != load_session {
            return Err(PayloadSendError::StaleToken);
        }
        if !matches!(
            (self.direction, direction),
            (PayloadDirection::ToServer, PayloadDirection::ToServer)
                | (PayloadDirection::ToClient, PayloadDirection::ToClient)
        ) {
            return Err(PayloadSendError::WrongDirection);
        }
        if !self.phase.permits(phase) {
            return Err(PayloadSendError::WrongPhase { phase });
        }
        Ok(self.slot)
    }
}

/// The token of a payload of `T` a client sends its server. Only the
/// serverbound send service takes it, so sending it the other way does not
/// compile.
#[derive(Debug)]
// exhaustive: its fields are private, so mods cannot build or destructure it
// no wire: a compile-time binding of a value type; its erased PayloadToken is what crosses.
pub struct ToServer<T> {
    token: PayloadToken,
    value: PhantomData<fn(&T)>,
}

/// The token of a payload of `T` a server sends a client. Only the
/// clientbound send service takes it, so sending it the other way does not
/// compile.
#[derive(Debug)]
// exhaustive: its fields are private, so mods cannot build or destructure it
// no wire: a compile-time binding of a value type; its erased PayloadToken is what crosses.
pub struct ToClient<T> {
    token: PayloadToken,
    value: PhantomData<fn(&T)>,
}

macro_rules! typed_token {
    ($name:ident, $direction:ident) => {
        impl<T> $name<T> {
            /// Binds `token` to `T`; None when it travels the other way.
            /// Registration checks that `T` is the declaration's value type.
            // hidden: a game-side helper, not part of the mod surface
            #[doc(hidden)]
            pub const fn bind(token: PayloadToken) -> Option<Self> {
                match token.direction {
                    PayloadDirection::$direction => Some(Self {
                        token,
                        value: PhantomData,
                    }),
                    _ => None,
                }
            }

            /// The phases the payload is declared for.
            pub const fn phase(&self) -> PayloadPhase {
                self.token.phase
            }

            /// The token with `T` erased, as the send service takes it.
            // hidden: the typed send helper's delegation, not a mod-facing call
            #[doc(hidden)]
            pub const fn erased(&self) -> PayloadToken {
                self.token
            }
        }

        impl<T> Clone for $name<T> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<T> Copy for $name<T> {}
    };
}

typed_token!(ToServer, ToServer);
typed_token!(ToClient, ToClient);

/// The client's send service: encoded payloads to the server. Object-safe;
/// the typed send helper encodes a value and delegates to this one erased
/// operation.
pub trait ServerboundPayloads {
    /// Queues `body`, the encoded value of `token`'s payload, for the
    /// server. Ok means accepted for sending, not delivered.
    fn send_erased(&mut self, token: PayloadToken, body: &[u8]) -> Result<(), PayloadSendError>;
}

/// The server's send service: encoded payloads to one player's client.
/// Object-safe; the typed send helper encodes a value and delegates to this
/// one erased operation.
pub trait ClientboundPayloads {
    /// Queues `body`, the encoded value of `token`'s payload, for `player`'s
    /// client. Ok means accepted for sending, not delivered.
    fn send_erased(
        &mut self,
        player: u128,
        token: PayloadToken,
        body: &[u8],
    ) -> Result<(), PayloadSendError>;
}

// Both services stay usable behind `dyn`: a generic method on either stops this compiling.
const _: fn(&mut dyn ServerboundPayloads, &mut dyn ClientboundPayloads) = |_, _| {};

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION: u64 = 41;

    fn token(phase: PayloadPhase, direction: PayloadDirection) -> PayloadToken {
        PayloadToken::issue(9, SESSION, phase, direction)
    }

    #[test]
    fn an_envelope_fits_up_to_vanillas_capacity_each_way() {
        let too_large = |size, limit| Err(PayloadSendError::PayloadTooLarge { size, limit });
        let to_server = PayloadDirection::ToServer;
        assert_eq!(to_server.admit_envelope(32_767), Ok(()));
        assert_eq!(to_server.admit_envelope(32_768), too_large(32_768, 32_767));
        let to_client = PayloadDirection::ToClient;
        assert_eq!(to_client.admit_envelope(1_048_576), Ok(()));
        assert_eq!(
            to_client.admit_envelope(1_048_577),
            too_large(1_048_577, 1_048_576)
        );
    }

    #[test]
    fn a_token_sends_only_its_own_way_in_its_phases_and_session() {
        use ConnectionPhase::{Configuration, Play};
        use PayloadDirection::{ToClient, ToServer};
        let play = token(PayloadPhase::Play, ToServer);
        assert_eq!(play.admit(ToServer, SESSION, Play), Ok(9));
        assert_eq!(
            play.admit(ToClient, SESSION, Play),
            Err(PayloadSendError::WrongDirection)
        );
        assert_eq!(
            play.admit(ToServer, SESSION, Configuration),
            Err(PayloadSendError::WrongPhase {
                phase: Configuration
            })
        );
        // A token of another load session is refused whatever else matches.
        assert_eq!(
            play.admit(ToServer, SESSION + 1, Play),
            Err(PayloadSendError::StaleToken)
        );
        let common = token(PayloadPhase::Common, ToClient);
        assert_eq!(common.admit(ToClient, SESSION, Configuration), Ok(9));
        assert_eq!(common.admit(ToClient, SESSION, Play), Ok(9));
        let configuration = token(PayloadPhase::Configuration, ToClient);
        assert_eq!(
            configuration.admit(ToClient, SESSION, Play),
            Err(PayloadSendError::WrongPhase { phase: Play })
        );
    }

    #[test]
    fn a_typed_token_binds_only_its_direction() {
        let to_server = token(PayloadPhase::Play, PayloadDirection::ToServer);
        let to_client = token(PayloadPhase::Play, PayloadDirection::ToClient);
        assert_eq!(
            ToServer::<u32>::bind(to_server).map(|t| t.erased()),
            Some(to_server)
        );
        assert!(ToServer::<u32>::bind(to_client).is_none());
        assert_eq!(
            ToClient::<u32>::bind(to_client).map(|t| t.erased()),
            Some(to_client)
        );
        assert!(ToClient::<u32>::bind(to_server).is_none());
    }
}
