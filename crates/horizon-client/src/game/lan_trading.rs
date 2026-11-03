//! Trading for LAN guests: the villager screen a guest opens runs the same
//! merchant menu as the host's (`horizon_client::menu`, `Kind::Merchant`),
//! with vanilla's packets: OpenScreen (`minecraft:merchant`), MerchantOffers,
//! the select trade packet, container clicks and the content sync after each.
use super::{
    Game, lan::LanHost, lan_actions::guest_within_entity_range, stations::Kind as StationKind,
    trading::MerchantState,
};
use horizon_client::{
    items::ItemStack,
    menu::{Env, Input, Kind, Menu},
    merchant,
};
use horizon_protocol::packets::play::{self, Clientbound as C};

/// A guest's open trading screen.
pub(super) struct GuestMerchant {
    pub(super) window: i32,
    state: i32,
    /// The payment slots and the result.
    slots: [Option<ItemStack>; 3],
    pub(super) merchant: Box<MerchantState>,
    /// The offers the client was last sent.
    sent: Vec<merchant::MerchantOffer>,
    sent_level: i32,
    sent_xp: i32,
}

/// What a trade hook reads of the merchant while the hub is out of the game:
/// the trade as the click left it. Hooks cannot write to it (a merchant edit
/// inside the trade's own hook is refused).
pub(super) struct TradeView {
    pub(super) mob: u32,
    pub(super) player: u128,
    pub(super) player_entity: u32,
    pub(super) offers: Vec<merchant::MerchantOffer>,
    pub(super) xp: i32,
    pub(super) level: i32,
}

impl GuestMerchant {
    /// The view of this trade for a hook, built only when a mod is loaded.
    fn view(&self, guest: &horizon_client::lan::guest::Guest) -> Option<TradeView> {
        if horizon_client::mod_api::mods().is_empty() {
            return None;
        }
        let s = &self.merchant.session;
        Some(TradeView {
            mob: self.merchant.mob?,
            player: guest.uuid,
            player_entity: u32::try_from(guest.entity_id).ok()?,
            offers: s.offers.clone(),
            xp: s.xp,
            level: s.level,
        })
    }
}

/// A guest's UUID as gossip targets it.
fn uuid_words(uuid: u128) -> [i32; 4] {
    [
        (uuid >> 96) as i32,
        (uuid >> 64) as i32,
        (uuid >> 32) as i32,
        uuid as i32,
    ]
}

impl Game<'_> {
    /// A trading screen the guest's interaction opened (it opened as a
    /// station while the guest acted): handed to the guest.
    pub(super) fn lan_open_merchant(
        &mut self,
        host: &mut LanHost,
        conn: u32,
        mut state: MerchantState,
    ) {
        let Some(gi) = host.hub.guests.iter().position(|g| g.conn == conn) else {
            return;
        };
        let uuid = uuid_words(host.hub.guests[gi].uuid);
        if let Some(id) = state.mob
            && state.session.villager
        {
            // startTrading with the guest's reputation. The session already
            // captured their Hero effect while they acted as the player.
            state.session.reputation = self.villager_reputation_of(id, uuid);
            state.session.update_special_prices();
        }
        let title = self.merchant_display_name(&state);
        if !self.guest_may_open(host, conn, "minecraft:merchant", None) {
            self.merchant_closed(&mut state);
            return;
        }
        // ServerPlayer.openMenu closes the menu that was open.
        self.guest_close_all(host, conn);
        let Some(window) = host.hub.open_merchant_window(conn, title) else {
            self.merchant_closed(&mut state);
            return;
        };
        let mut gm = GuestMerchant {
            window,
            state: 0,
            slots: [None, None, None],
            merchant: Box::new(state),
            sent: Vec::new(),
            sent_level: 0,
            sent_xp: 0,
        };
        self.lan_send_offers(host, conn, &mut gm, true);
        self.lan_send_merchant_content(host, conn, &mut gm);
        host.play.entry(conn).or_default().merchant = Some(gm);
    }
    /// `Entity.getDisplayName` of the merchant, the screen's title: its name
    /// (custom, else the profession's) with the show_entity hover event and
    /// its UUID as insertion.
    fn merchant_display_name(&mut self, state: &MerchantState) -> horizon_world::nbt::Tag {
        use horizon_world::nbt::Tag;
        let s = &state.session;
        let (kind, key) = if s.villager {
            let p = s.profession.trim_start_matches("minecraft:");
            (
                "minecraft:villager",
                format!("entity.minecraft.villager.{p}"),
            )
        } else {
            (
                "minecraft:wandering_trader",
                "entity.minecraft.wandering_trader".to_owned(),
            )
        };
        let name = match &state.name {
            Some(n) => horizon_protocol::text::literal(n),
            None => Tag::compound([("translate", Tag::String(key.as_str().into()))]),
        };
        let uuid = state.mob.map_or([0; 4], |id| self.mob_uuid(id));
        let w = uuid.map(|v| v as u32);
        let text = format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:04x}{:08x}",
            w[0],
            w[1] >> 16,
            w[1] & 0xffff,
            w[2] >> 16,
            w[2] & 0xffff,
            w[3]
        );
        let mut out = match name.clone() {
            Tag::Compound(c) => c,
            other => {
                let mut c = horizon_world::nbt::Compound::new();
                c.insert("text".into(), other);
                c
            }
        };
        out.insert(
            "hover_event".into(),
            Tag::compound([
                ("action", Tag::String("show_entity".into())),
                ("id", Tag::String(kind.into())),
                ("uuid", Tag::IntArray(uuid.to_vec())),
                ("name", name),
            ]),
        );
        out.insert("insertion".into(), Tag::String(text.as_str().into()));
        Tag::Compound(out)
    }
    /// The screen title: the custom name, else the profession (the client
    /// adds the level itself).
    /// `ServerPlayer.sendMerchantOffers`, when the offers changed.
    fn lan_send_offers(
        &mut self,
        host: &mut LanHost,
        conn: u32,
        gm: &mut GuestMerchant,
        force: bool,
    ) {
        let s = &gm.merchant.session;
        if !force && s.offers == gm.sent && s.level == gm.sent_level && s.xp == gm.sent_xp {
            return;
        }
        let offers: Vec<play::NetOffer> = s
            .offers
            .iter()
            .filter_map(|o| {
                let cost = |st: &ItemStack, hub: &mut horizon_client::lan::hub::Hub| {
                    hub.stack(&Some(st.clone())).map(play::NetCost::from)
                };
                Some(play::NetOffer {
                    cost_a: cost(&o.base_cost_a.stack, &mut host.hub)?,
                    result: host.hub.stack(&Some(o.result.clone()))?,
                    cost_b: match &o.cost_b {
                        Some(b) => Some(cost(&b.stack, &mut host.hub)?),
                        None => None,
                    },
                    out_of_stock: o.out_of_stock(),
                    uses: o.uses,
                    max_uses: o.max_uses,
                    xp: o.xp,
                    special_price: o.special_price_diff,
                    price_multiplier: o.price_multiplier,
                    demand: o.demand,
                })
            })
            .collect();
        host.hub.send_to(
            conn,
            &C::MerchantOffers(play::MerchantOffers {
                container: gm.window,
                offers,
                level: s.level,
                xp: s.xp,
                show_progress: s.villager,
                can_restock: s.villager,
            }),
        );
        gm.sent = s.offers.clone();
        gm.sent_level = s.level;
        gm.sent_xp = s.xp;
    }
    fn lan_send_merchant_content(&mut self, host: &mut LanHost, conn: u32, gm: &mut GuestMerchant) {
        let Some(gi) = host.hub.guests.iter().position(|g| g.conn == conn) else {
            return;
        };
        let mut items: Vec<Option<ItemStack>> = gm.slots.to_vec();
        let g = &host.hub.guests[gi];
        items.extend((9..36).chain(0..9).map(|i| g.inventory[i].clone()));
        let carried = g.carried.clone();
        let net: Vec<_> = items.iter().map(|s| host.hub.stack(s)).collect();
        let carried = host.hub.stack(&carried);
        gm.state = (gm.state + 1) & 0x7FFF;
        host.hub.send_to(
            conn,
            &C::ContainerSetContent(play::ContainerSetContent {
                container: gm.window,
                state: gm.state,
                items: net,
                carried,
            }),
        );
    }
    /// Runs `op` on the guest's merchant menu (their inventory copied in and
    /// back), then syncs the client. False when the guest has no such window.
    fn lan_merchant_op(
        &mut self,
        host: &mut LanHost,
        conn: u32,
        window: Option<i32>,
        op: impl FnOnce(&mut Menu, &Env),
    ) -> bool {
        let Some(mut gm) = host.play.get_mut(&conn).and_then(|p| p.merchant.take()) else {
            return false;
        };
        if window.is_some_and(|w| w != gm.window) {
            host.play.entry(conn).or_default().merchant = Some(gm);
            return false;
        }
        let Some(gi) = host.hub.guests.iter().position(|g| g.conn == conn) else {
            return false;
        };
        let mut session = gm.merchant.session.clone();
        self.merchant_sync_in(&mut session, gm.merchant.mob);
        let next_map = self.peek_map_id();
        let tag = |t: &str| self.structure_tag(t);
        let maps = super::trading::GameMaps {
            terrain: self.end.locator(),
            structure_tag: &tag,
            made: Default::default(),
            next_id: std::cell::Cell::new(next_map),
        };
        let trade_env = merchant::TradeEnv {
            trades: &self.stations().trades,
            enchantments: &self.stations().enchantments,
            maps: Some(&maps),
        };
        let env = Env {
            player: Some(host.hub.guests[gi].uuid),
            player_entity_id: Some(host.hub.guests[gi].entity_id),
            side: horizon_client::menu::LogicalSide::Server,
            items: self.items(),
            recipes: self.recipes(),
            brewing: &self.stations().brewing,
            banners: &self.stations().banners,
            station_recipes: &self.stations().recipes,
            enchantments: &self.stations().enchantments,
            names: &self.pack.names,
            trade: Some(&trade_env),
            maps: None,
        };
        let mut menu = Menu::new(Kind::Merchant);
        {
            let g = &host.hub.guests[gi];
            for i in 0..36 {
                menu.set_player_item(i, g.inventory[i].clone());
            }
            menu.set_player_item(40, g.offhand.clone());
            menu.selected = g.selected;
            menu.creative = g.creative();
            menu.carried = g.carried.clone();
        }
        for (i, s) in gm.slots.iter().enumerate() {
            menu.set_slot(&env, i, s.clone());
        }
        menu.merchant = Some(session);
        op(&mut menu, &env);
        let made = maps.made.take();
        let session = menu.merchant.take().expect("attached");
        let dropped = std::mem::take(&mut menu.dropped);
        gm.slots = [
            menu.slot(0).cloned(),
            menu.slot(1).cloned(),
            menu.slot(2).cloned(),
        ];
        {
            let g = &mut host.hub.guests[gi];
            for i in 0..36 {
                g.inventory[i] = menu.player_item(i).cloned();
            }
            g.offhand = menu.player_item(40).cloned();
            g.carried = menu.carried.take();
        }
        self.register_exploration_maps(made);
        let (session, traded) = self.merchant_apply(session, gm.merchant.mob);
        gm.merchant.session = session;
        if !traded.is_empty() {
            self.trade_view = host.hub.guests.get(gi).and_then(|g| gm.view(g));
            // The guest's own statistic and advancements.
            if let Some(gi) = host.hub.guests.iter().position(|g| g.conn == conn) {
                let g = &host.hub.guests[gi];
                if let Some(entity_id) = gm.merchant.mob {
                    self.native_player_trade(g.uuid, &g.name, entity_id);
                }
            }
            self.as_guest(host, conn, |g| g.merchant_traded(gm.merchant.mob, &traded));
            self.trade_view = None;
        }
        self.drop_guest_items(host, gi, dropped);
        self.lan_send_offers(host, conn, &mut gm, false);
        self.lan_send_merchant_content(host, conn, &mut gm);
        host.play.entry(conn).or_default().merchant = Some(gm);
        true
    }
    /// A container click on the guest's trading window.
    pub(super) fn lan_merchant_click(
        &mut self,
        host: &mut LanHost,
        conn: u32,
        p: &play::ContainerClick,
    ) -> bool {
        let input = match p.input {
            0 => Input::Pickup,
            1 => Input::QuickMove,
            2 => Input::Swap,
            3 => Input::Clone,
            4 => Input::Throw,
            5 => Input::QuickCraft,
            _ => Input::PickupAll,
        };
        let (slot, button) = (i32::from(p.slot), i32::from(p.button));
        self.lan_merchant_op(host, conn, Some(p.container), |menu, env| {
            let _ = menu.click(env, slot, button, input);
        })
    }
    /// ServerboundSelectTradePacket: the selection hint, then tryMoveItems.
    pub(super) fn lan_select_trade(&mut self, host: &mut LanHost, conn: u32, index: i32) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        let Some(guest) = host.hub.guests.iter().find(|g| g.conn == conn) else {
            return;
        };
        let (uuid, name) = (guest.uuid, guest.name.clone());
        if self.lan_merchant_op(host, conn, None, |menu, env| {
            menu.merchant_select(env, index)
        }) {
            let gm = host.play.get(&conn).and_then(|p| p.merchant.as_ref());
            let villager = gm.and_then(|m| m.merchant.mob);
            self.trade_view = gm.zip(host.hub.guest(conn)).and_then(|(gm, g)| gm.view(g));
            self.native_trade_select(uuid, &name, villager, index);
            self.trade_view = None;
        }
    }
    /// The guest closed the trading window (or it is closed for them):
    /// MerchantMenu.removed, then the villager gets its state back.
    pub(super) fn lan_merchant_closed(
        &mut self,
        host: &mut LanHost,
        conn: u32,
        window: Option<i32>,
    ) -> bool {
        if !self.lan_merchant_op(host, conn, window, |menu, env| menu.merchant_close(env)) {
            return false;
        }
        let Some(mut gm) = host.play.get_mut(&conn).and_then(|p| p.merchant.take()) else {
            return false;
        };
        let uuid = host
            .hub
            .guests
            .iter()
            .find(|g| g.conn == conn)
            .map(|g| uuid_words(g.uuid));
        if std::mem::take(&mut gm.merchant.session.last_traded)
            && let (Some(id), Some(uuid)) = (gm.merchant.mob, uuid)
        {
            self.villager_traded_by(id, uuid);
        }
        self.merchant_closed(&mut gm.merchant);
        true
    }
    /// `Merchant.stillValid` for a trader: it is alive and the guest is within
    /// the entity interaction range plus 4 of its box.
    pub(super) fn lan_merchant_valid(
        &self,
        g: &horizon_client::lan::guest::Guest,
        id: u32,
    ) -> bool {
        self.mobs
            .iter()
            .find(|m| m.id == id)
            .is_some_and(|m| m.alive() && guest_within_entity_range(g, &m.bounds(), 4.0))
    }
    /// `Villager.customServerAiStep` for the guests' trading villagers: the
    /// TRADE reputation event, then the prices follow.
    pub(super) fn lan_merchant_tick(&mut self, host: &mut LanHost) {
        // `AbstractVillager.stillValid`: the trade screen closes once the trader
        // is gone or dead, or the guest is more than 4 blocks past its reach.
        let far: Vec<(u32, i32)> = host
            .play
            .iter()
            .filter_map(|(conn, p)| {
                let gm = p.merchant.as_ref()?;
                let id = gm.merchant.mob?;
                let g = host
                    .hub
                    .guests
                    .iter()
                    .find(|g| g.conn == *conn && host.hub.here(g))?;
                (!self.lan_merchant_valid(g, id)).then_some((*conn, gm.window))
            })
            .collect();
        for (conn, window) in far {
            if self.lan_merchant_closed(host, conn, Some(window)) {
                host.hub.send_to(
                    conn,
                    &C::ContainerClose(play::ContainerClose { container: window }),
                );
            }
        }
        let conns: Vec<u32> = host
            .play
            .iter()
            .filter(|(_, p)| {
                p.merchant
                    .as_ref()
                    .is_some_and(|m| m.merchant.session.villager && m.merchant.session.last_traded)
            })
            .map(|(c, _)| *c)
            .collect();
        for conn in conns {
            let Some(uuid) = host
                .hub
                .guests
                .iter()
                .find(|g| g.conn == conn && host.hub.here(g))
                .map(|g| uuid_words(g.uuid))
            else {
                continue;
            };
            let Some(gm) = host.play.get_mut(&conn).and_then(|p| p.merchant.as_mut()) else {
                continue;
            };
            gm.merchant.session.last_traded = false;
            let Some(id) = gm.merchant.mob else { continue };
            self.villager_traded_by(id, uuid);
            let reputation = self.villager_reputation_of(id, uuid);
            self.lan_merchant_op(host, conn, None, |menu, env| {
                if let Some(s) = menu.merchant.as_mut() {
                    s.reputation = reputation;
                    s.update_special_prices();
                }
                menu.merchant_update(env);
            });
        }
    }
    /// Takes a station a guest's action opened: a trading screen or a
    /// workstation menu goes to the guest (game/lan_stations.rs).
    pub(super) fn lan_station_opened(
        &mut self,
        host: &mut LanHost,
        conn: u32,
        station: super::stations::Station,
    ) {
        let mut station = station;
        if station.kind == StationKind::Merchant
            && let Some(m) = station.merchant.take()
        {
            self.lan_open_merchant(host, conn, *m);
            return;
        }
        self.lan_open_station(host, conn, &station);
    }
}

impl Game<'_> {
    /// The guest whose trading screen is open on mob `mob`.
    pub(super) fn lan_trade_conn(&self, mob: u32) -> Option<u32> {
        let host = self.lan.as_ref()?;
        host.play
            .iter()
            .find(|(_, p)| {
                p.merchant
                    .as_ref()
                    .is_some_and(|m| m.merchant.mob == Some(mob))
            })
            .map(|(conn, _)| *conn)
    }
    /// The live session of a guest's open trading screen, whose offers,
    /// experience and level are newer than the mob's until the screen closes.
    pub(super) fn lan_trade_session(&self, conn: u32) -> Option<&merchant::Session> {
        let gm = self.lan.as_ref()?.play.get(&conn)?.merchant.as_ref()?;
        Some(&gm.merchant.session)
    }
    /// The guest's player UUID.
    pub(super) fn lan_trade_player(&self, conn: u32) -> Option<u128> {
        Some(self.lan.as_ref()?.hub.guest(conn)?.uuid)
    }
    /// Changes the session of a guest's open trading screen and sends the
    /// client its offers and result slot again. The merchant's own random is
    /// not drawn: the update sound is held back.
    pub(super) fn lan_trade_edit(
        &mut self,
        conn: u32,
        edit: impl FnOnce(&mut merchant::Session),
    ) -> bool {
        let Some(mut host) = self.lan.take() else {
            return false;
        };
        let done = self.lan_merchant_op(&mut host, conn, None, |menu, env| {
            let Some(session) = menu.merchant.as_mut() else {
                return;
            };
            edit(session);
            session.selection_hint = session
                .selection_hint
                .min(session.offers.len().saturating_sub(1));
            let sound_time = std::mem::replace(&mut session.ambient_sound_time, -80);
            menu.merchant_update(env);
            if let Some(session) = menu.merchant.as_mut() {
                session.ambient_sound_time = sound_time;
            }
        });
        if done {
            self.lan_trade_sequences(&mut host, conn);
        }
        self.lan = Some(host);
        done
    }
    /// Lends the open session the world sequences of the levels above its
    /// level, and gives back the ones it no longer needs, so a level-up after
    /// the level was changed still rolls its trade set.
    fn lan_trade_sequences(&mut self, host: &mut LanHost, conn: u32) {
        let Some(gm) = host.play.get_mut(&conn).and_then(|p| p.merchant.as_mut()) else {
            return;
        };
        let session = &mut gm.merchant.session;
        if !session.villager {
            return;
        }
        let needed = merchant::Session::sequences_needed(
            &self.stations().trades,
            &session.profession,
            session.level,
        );
        let stale: Vec<String> = session
            .sequences
            .keys()
            .filter(|id| !needed.contains(id))
            .cloned()
            .collect();
        for id in stale {
            if let Some(random) = session.sequences.remove(&id) {
                self.rand_sequences.give_back(id, random);
            }
        }
        for id in needed {
            if let std::collections::btree_map::Entry::Vacant(slot) = session.sequences.entry(id) {
                let random = self.rand_sequences.lend(slot.key(), self.level.seed);
                slot.insert(random);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_entity_refs::tests::with_game;
    use super::super::native_merchant_refs::tests::{offer, spawn};
    use super::*;
    use crate::game::lan::LanSettings;
    use horizon_client::{
        lan::{Joined, NetConfig, PlayerSender, guest::Guest, hub::HubConfig},
        merchant::{MerchantOffer, Trader},
    };
    use horizon_protocol::packets::configuration::ClientInformation;
    use std::sync::{
        Arc,
        atomic::{AtomicI32, AtomicUsize},
    };

    /// A hub with one guest (conn 12) trading with a farmer villager (mob 40)
    /// at `level`; the guest's channel comes back with the villager.
    fn open_trade(
        game: &mut Game<'_>,
        level: i32,
    ) -> (
        horizon_client::mod_api::MerchantRef,
        std::sync::mpsc::Receiver<horizon_client::lan::Out>,
        MerchantOffer,
    ) {
        let settings = LanSettings {
            port: 0,
            guest_commands: false,
            force_game_mode: false,
        };
        let net = NetConfig {
            listen: false,
            announce: false,
            lod_distance: 0,
            ..NetConfig::lan(0, String::new(), 2)
        };
        let config = HubConfig {
            default_game_mode: 0,
            guest_commands: false,
            force_game_mode: false,
            hardcore: false,
            view_distance: 2,
            simulation_distance: 2,
            dimension: game.dim.id().to_owned(),
            allow_flight: true,
            seed: 0,
            max_players: 2,
            integrated: true,
            levels: game.level_shapes,
        };
        game.lan_host(settings, net, config).unwrap();
        let mut host = game.lan.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut guest = Guest::new(
            Joined {
                conn: 12,
                uuid: 12,
                name: "guest12".into(),
                properties: Vec::new(),
                info: ClientInformation::new(8),
                brand: String::new(),
                sender: PlayerSender::new(tx, Arc::new(AtomicUsize::new(0))),
                latency: Arc::new(AtomicI32::new(0)),
                address: "127.0.0.1:1".parse().unwrap(),
            },
            12,
            [3.0, 64.0, 5.0],
            10,
        );
        guest.placed = true;
        host.hub.guests.push(guest);

        let villager = spawn(game, 40, "minecraft:villager");
        game.mobs[0].extra.variant2 = 5;
        let profession = "minecraft:farmer".to_owned();
        let first = offer("minecraft:wheat", "minecraft:emerald");
        game.mobs[0].extra.trade = Some(Box::new(Trader {
            offers: vec![first.clone()],
            ..Trader::default()
        }));
        game.mobs[0].extra.flag2 = true;
        let mut session = merchant::Session::new(vec![first.clone()], true);
        session.level = level;
        session.profession = profession;
        session.xp = 0;
        session.variant = "minecraft:plains".into();
        for id in
            merchant::Session::sequences_needed(&game.stations().trades, &session.profession, level)
        {
            let random = game.rand_sequences.lend(&id, game.level.seed);
            session.sequences.insert(id, random);
        }
        let gm = GuestMerchant {
            window: 3,
            state: 0,
            slots: [None, None, None],
            merchant: Box::new(MerchantState {
                session: Box::new(session),
                scroll: 0,
                dragging: false,
                focused: None,
                mob: Some(40),
                name: None,
            }),
            sent: vec![first.clone()],
            sent_level: level,
            sent_xp: 0,
        };
        host.play.entry(12).or_default().merchant = Some(gm);
        game.lan = Some(host);
        (villager, rx, first)
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_merchant_edit_reaches_the_guest_trading_with_it() {
        with_game(|game| {
            let (villager, rx, first) = open_trade(game, 1);

            // MerchantOffers packets (id 53) the guest was sent since last asked.
            let sent_offers = || {
                let mut n = 0;
                while let Ok(out) = rx.try_recv() {
                    let bodies = match out {
                        horizon_client::lan::Out::Packet(body) => vec![body],
                        horizon_client::lan::Out::Packets(bodies) => bodies,
                        _ => Vec::new(),
                    };
                    n += bodies.iter().filter(|b| b.first() == Some(&53)).count();
                }
                n
            };
            sent_offers();
            let player = villager.trading_player(game).unwrap();
            assert_eq!(player.uuid(), 12);
            let second = offer("minecraft:coal", "minecraft:diamond");
            assert!(villager.add_offer(game, second.clone()));
            assert_eq!(sent_offers(), 1);
            let mut changed = first.clone();
            changed.uses = 9;
            assert!(villager.set_offer(game, 0, changed.clone()));
            assert!(villager.set_xp(game, 40));
            assert!(villager.set_level(game, 3));
            // The profession cannot change under an open screen.
            assert!(!villager.set_profession(game, "minecraft:cleric"));
            assert!(!villager.set_offer(game, 7, changed.clone()));
            assert_eq!(sent_offers(), 3);
            // The mob still has the old offers; the screen has the new ones.
            assert_eq!(
                game.mobs[0].extra.trade.as_ref().unwrap().offers,
                vec![first]
            );
            let live = vec![changed.clone(), second.clone()];
            assert_eq!(villager.offers(game), Some(live.clone()));
            assert_eq!(villager.xp(game), Some(40));
            assert_eq!(villager.level(game), Some(3));
            {
                let gm = game.lan.as_ref().unwrap().play[&12]
                    .merchant
                    .as_ref()
                    .unwrap();
                assert_eq!(gm.sent, live);
                assert_eq!(gm.sent_level, 3);
                assert_eq!(gm.merchant.session.offers, live);
            }
            assert!(villager.remove_offer(game, 1));
            let gm = game.lan.as_ref().unwrap().play[&12]
                .merchant
                .as_ref()
                .unwrap();
            assert_eq!(gm.sent, vec![changed.clone()]);
            assert_eq!(sent_offers(), 1);

            // Closing the screen gives the mob the edited trades.
            let mut host = game.lan.take().unwrap();
            let mut gm = host.play.get_mut(&12).unwrap().merchant.take().unwrap();
            game.merchant_closed(&mut gm.merchant);
            game.lan = Some(host);
            let trade = game.mobs[0].extra.trade.as_ref().unwrap();
            assert_eq!((trade.offers.clone(), trade.xp), (vec![changed], 40));
            assert_eq!(game.mobs[0].extra.variant3, 3);
            assert_eq!(villager.trading_player(game), None);
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_lowered_level_still_levels_up_with_the_next_trade_set() {
        with_game(|game| {
            let (villager, _rx, first) = open_trade(game, 3);
            let before = game.lan_trade_session(12).unwrap().sequences.len();
            assert_eq!(before, 2, "levels 4 and 5 were lent");
            assert!(villager.set_level(game, 1));
            assert!(villager.set_xp(game, 5));
            assert_eq!(game.lan_trade_session(12).unwrap().sequences.len(), 4);
            // The trade makes 12 experience: past level 1's 10, so level 2 is rolled.
            let mut host = game.lan.take().unwrap();
            let gm = host.play.get_mut(&12).unwrap().merchant.as_mut().unwrap();
            let items = game.items();
            let trade_env = merchant::TradeEnv {
                trades: &game.stations().trades,
                enchantments: &game.stations().enchantments,
                maps: None,
            };
            let session = &mut gm.merchant.session;
            session.origin = [0.0; 3];
            assert!(session.notify_trade(0, Some(&trade_env), items));
            assert_eq!(session.level, 2);
            assert!(session.offers.len() > 1, "level 2 offers were added");
            assert_eq!(session.offers[0].uses, first.uses + 1);
            game.lan = Some(host);
            // Level 7: nothing more to lend, and no further level-up.
            assert!(villager.set_level(game, 7));
            assert_eq!(villager.level(game), Some(7));
            assert!(game.lan_trade_session(12).unwrap().sequences.is_empty());
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_type_change_reaches_the_open_session_before_a_level_up() {
        with_game(|game| {
            let (villager, _rx, _) = open_trade(game, 4);
            assert!(villager.set_villager_type(game, "minecraft:swamp"));
            assert_eq!(
                game.lan_trade_session(12).unwrap().variant,
                "minecraft:swamp"
            );
            assert_eq!(game.mobs[0].extra.variant, 5);
            assert_eq!(
                villager.villager_type(game).as_deref(),
                Some("minecraft:swamp")
            );
        });
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_trade_hook_reads_the_live_trade_and_cannot_write_it() {
        with_game(|game| {
            let (villager, _rx, first) = open_trade(game, 2);
            let second = offer("minecraft:coal", "minecraft:diamond");
            assert!(villager.add_offer(game, second.clone()));
            assert!(villager.set_xp(game, 30));
            // A hook fires with the hub out of the game and the view set.
            let host = game.lan.take().unwrap();
            let gm = host.play[&12].merchant.as_ref().unwrap();
            let guest = host.hub.guest(12).unwrap();
            game.trade_view = Some(TradeView {
                mob: 40,
                player: guest.uuid,
                player_entity: guest.entity_id as u32,
                offers: gm.merchant.session.offers.clone(),
                xp: gm.merchant.session.xp,
                level: gm.merchant.session.level,
            });
            assert_eq!(villager.offers(game), Some(vec![first.clone(), second]));
            assert_eq!(
                (villager.xp(game), villager.level(game)),
                (Some(30), Some(2))
            );
            let player = villager.trading_player(game).unwrap();
            assert_eq!(player.uuid(), 12);
            // The mob alone has the old state, so these reads came from the view.
            assert_eq!(
                game.mobs[0].extra.trade.as_ref().unwrap().offers,
                vec![first]
            );
            // Writes inside the trade's own hook are refused.
            assert!(!villager.add_offer(game, offer("minecraft:wheat", "minecraft:emerald")));
            assert!(!villager.set_xp(game, 1));
            assert!(!villager.set_villager_type(game, "minecraft:desert"));
            game.trade_view = None;
            game.lan = Some(host);
        });
    }
}
