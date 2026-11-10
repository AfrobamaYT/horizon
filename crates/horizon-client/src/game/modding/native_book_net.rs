//! The dedicated custom-book channel uses the negotiated mod-transport capability.
use super::Game;
use horizon_client::recipe_book::{
    settings_codec,
    settings_sync::{Authority, CHANNEL, ClientView, Message},
};
use horizon_protocol::packets::{
    configuration::CustomPayload,
    play::{Clientbound as C, Serverbound as S},
};
use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicU64, Ordering},
};
static EPOCH: AtomicU64 = AtomicU64::new(1);
fn registered() -> BTreeSet<String> {
    mod_api::RecipeBookTypeKey::registered()
        .into_iter()
        .map(|key| key.as_str().to_owned())
        .collect()
}
impl Game<'_> {
    pub(super) fn reset_book_view(&mut self) {
        self.native_net.book_view = Default::default();
        self.native_net.book_request = 0;
    }
    pub(super) fn is_book_or_mod_channel(&self, channel: &str) -> bool {
        channel == CHANNEL || channel == horizon_protocol::mod_net::CHANNEL
    }
    pub(super) fn remote_book_or_mod_payload(&mut self, channel: &str, data: &[u8]) {
        if channel == CHANNEL {
            self.remote_book_settings(data);
        } else {
            self.remote_native_net(data);
        }
    }

    pub(super) fn clear_guest_book_connection(&mut self, conn: u32) {
        self.guest_combat.remove(&conn);
        self.native_net.book_connections.remove(&conn);
    }
    pub(super) fn notify_vanilla_book_settings(
        &mut self,
        uuid: u128,
        name: &str,
        book: i32,
        open: bool,
        filtering: bool,
    ) {
        let book = match book {
            0 => mod_api::RecipeBookIdentity::Crafting,
            1 => mod_api::RecipeBookIdentity::Furnace,
            2 => mod_api::RecipeBookIdentity::BlastFurnace,
            3 => mod_api::RecipeBookIdentity::Smoker,
            _ => return,
        };
        self.native_player_recipe_book_settings_change(uuid, name, book, open, filtering);
    }

    /// Authoritative view for a custom menu panel. None disables the panel
    /// until negotiation and the initial snapshot finish.
    pub fn custom_book_settings(
        &self,
        book: &mod_api::RecipeBookTypeKey,
    ) -> Option<mod_api::RecipeBookSettings> {
        let view = &self.native_net.book_view;
        if !view.ready || !view.contains(book.as_str()) {
            return None;
        }
        let flags = view.books.get(book.as_str());
        Some(mod_api::RecipeBookSettings {
            open: flags.open,
            filtering: flags.filtering,
        })
    }
    /// Queue intent without changing the authoritative or saved pair.
    pub fn request_custom_book_settings(
        &mut self,
        book: &mod_api::RecipeBookTypeKey,
        flags: mod_api::RecipeBookSettings,
    ) -> bool {
        if self.custom_book_settings(book).is_none() {
            return false;
        }
        let Some(number) = self.native_net.book_request.checked_add(1) else {
            return false;
        };
        let Some(epoch) = self.native_net.book_view.epoch else {
            return false;
        };
        let message = Message::Request {
            epoch,
            number,
            key: book.as_str().into(),
            flags: horizon_client::recipe_book::BookFlags {
                open: flags.open,
                filtering: flags.filtering,
            },
        };
        let Some(data) = settings_codec::encode(&message) else {
            return false;
        };
        self.native_net.book_request = number;
        self.send(S::CustomPayload(CustomPayload {
            channel: CHANNEL.into(),
            data,
        }));
        true
    }

    pub(super) fn sync_server_book_write(
        &mut self,
        player: mod_api::PlayerRef,
        book: &mod_api::RecipeBookTypeKey,
        flags: horizon_client::recipe_book::BookFlags,
    ) {
        let Some(host) = self.lan.as_ref() else {
            return;
        };
        let Some(guest) = host
            .hub
            .guests
            .iter()
            .find(|g| g.uuid == player.uuid() && g.placed)
        else {
            return;
        };
        let Some(authority) = self.native_net.book_connections.get_mut(&guest.conn) else {
            return;
        };
        if let Some(message) = authority.server_update(book.as_str(), flags) {
            if let Some(data) = settings_codec::encode(&message) {
                guest.send(&C::CustomPayload(CustomPayload {
                    channel: CHANNEL.into(),
                    data,
                }));
            }
        }
    }

    pub(super) fn begin_book_negotiation(&mut self) {
        self.native_net.book_view = ClientView::default();
        self.native_net.book_request = 0;
        if self.remote.as_ref().is_some_and(|r| r.mod_net && r.joined) {
            if let Some(data) =
                settings_codec::encode(&Message::Offer(registered().into_iter().collect()))
            {
                self.send(S::CustomPayload(CustomPayload {
                    channel: CHANNEL.into(),
                    data,
                }));
            }
        }
    }
    pub(super) fn guest_book_settings(&mut self, conn: u32, data: &[u8]) {
        let Some(message) = settings_codec::decode(data) else {
            return;
        };
        let Some(host) = self.lan.as_mut() else {
            return;
        };
        let connections = &mut self.native_net.book_connections;
        let changed = host
            .hub
            .guests
            .update_connection(conn, |guest| {
                if !guest.placed || guest.brand != "horizon" {
                    return None;
                }
                let mut changed = None;
                let response = match message {
                    Message::Offer(keys) => {
                        // A connection may negotiate once; repeated offers cannot reset
                        // replay protection or the rate-limit window.
                        if connections.contains_key(&conn) {
                            return None;
                        }
                        let authority = Authority::negotiate(
                            EPOCH.fetch_add(1, Ordering::Relaxed),
                            keys,
                            &registered(),
                        );
                        let snapshot = authority.snapshot(&guest.recipe_book.custom);
                        connections.insert(conn, authority);
                        snapshot
                    }
                    Message::Request {
                        epoch,
                        number,
                        key,
                        flags,
                    } => {
                        let Some(authority) = connections.get_mut(&conn) else {
                            return None;
                        };
                        let (reply, accepted) = authority.request(
                            &mut guest.recipe_book.custom,
                            epoch,
                            number,
                            &key,
                            flags,
                            std::time::Instant::now(),
                        );
                        if accepted {
                            changed = Some((guest.uuid, guest.name.clone(), key, flags));
                        }
                        reply
                    }
                    Message::Resnapshot => {
                        let Some(authority) = connections.get(&conn) else {
                            return None;
                        };
                        authority.snapshot(&guest.recipe_book.custom)
                    }
                    _ => return None,
                };
                if let Some(data) = settings_codec::encode(&response) {
                    guest.send(&C::CustomPayload(CustomPayload {
                        channel: CHANNEL.into(),
                        data,
                    }));
                }
                changed
            })
            .flatten();
        if let Some((uuid, name, id, flags)) = changed {
            if let Some(key) = mod_api::RecipeBookTypeKey::registered()
                .into_iter()
                .find(|key| key.as_str() == id)
            {
                self.native_player_recipe_book_settings_change(
                    uuid,
                    &name,
                    mod_api::RecipeBookIdentity::Custom(key),
                    flags.open,
                    flags.filtering,
                );
            }
        }
    }
    pub(super) fn remote_book_settings(&mut self, data: &[u8]) {
        let Some(message) = settings_codec::decode(data) else {
            return;
        };
        if !self.native_net.book_view.receive(message, &registered())
            && !self.native_net.book_view.ready
        {
            if let Some(data) = settings_codec::encode(&Message::Resnapshot) {
                self.send(S::CustomPayload(CustomPayload {
                    channel: CHANNEL.into(),
                    data,
                }));
            }
        }
    }
}
