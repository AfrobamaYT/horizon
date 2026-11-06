//! The `merchant_refs` area: host methods the game implements (`game/host/merchant_refs.rs`).
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `merchant_refs` area, implemented by the game and reached through [`ServerHost`].
pub trait MerchantRefsServerHost {
    /// Side: server.
    /// Narrows an entity to a villager or wandering trader, or None for
    /// another kind, a stale handle, or a client.
    fn merchant(&self, entity: EntityRef) -> Option<MerchantRef> {
        crate::phase::unprovided(
            "ServerHost::merchant",
            "merchant",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `merchant_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait MerchantRefsBackend {
    /// Side: server.
    /// `AbstractVillager.getOffers`, as they are now (a copy). Offers not
    /// rolled yet read as empty: they are rolled when a player first trades,
    /// or by `add_offer`; reading never draws a random.
    fn merchant_offers(&self, merchant: MerchantRef) -> Option<Vec<MerchantOffer>> {
        crate::phase::unprovided(
            "Backend::merchant_offers",
            "merchant_offers",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Merchant.getTradingPlayer`: the player whose trading screen is open on
    /// it. None when nobody trades, or the screen is the host window's.
    fn merchant_trading_player(&self, merchant: MerchantRef) -> Option<PlayerRef> {
        crate::phase::unprovided(
            "Backend::merchant_trading_player",
            "merchant_trading_player",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `VillagerData.profession`, a registry id (`minecraft:farmer`; none is
    /// `minecraft:none`). None for a wandering trader.
    fn merchant_profession(&self, merchant: MerchantRef) -> Option<String> {
        crate::phase::unprovided(
            "Backend::merchant_profession",
            "merchant_profession",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `VillagerData.level`, at least 1.
    fn merchant_level(&self, merchant: MerchantRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::merchant_level",
            "merchant_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `VillagerData.type`, a registry id (`minecraft:plains`).
    fn merchant_villager_type(&self, merchant: MerchantRef) -> Option<String> {
        crate::phase::unprovided(
            "Backend::merchant_villager_type",
            "merchant_villager_type",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Villager.getVillagerXp`.
    fn merchant_xp(&self, merchant: MerchantRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::merchant_xp",
            "merchant_xp",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `merchant_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait MerchantRefsBackendMut {
    /// Side: server.
    /// Replaces offer `index` (a player trading with it is sent the new
    /// offers). False for an index past the end, a stale handle, or an offer
    /// without a real item in `base_cost_a` or `result` (or `cost_b` when set).
    fn merchant_set_offer(
        &mut self,
        merchant: MerchantRef,
        index: u32,
        offer: MerchantOffer,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::merchant_set_offer",
            "merchant_set_offer",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Appends an offer, as `getOffers().add(offer)` does. Offers not rolled
    /// yet are rolled first, as `getOffers` does for a player's first trade
    /// (this draws the trade set's world random sequence, as that does).
    /// False for a stale handle or an invalid offer (see `set_offer`).
    fn merchant_add_offer(
        &mut self,
        merchant: MerchantRef,
        offer: MerchantOffer,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::merchant_add_offer",
            "merchant_add_offer",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Removes offer `index`, the ones after it moving up. False for an index
    /// past the end or a stale handle.
    fn merchant_remove_offer(&mut self, merchant: MerchantRef, index: u32) -> bool {
        crate::phase::unprovided(
            "BackendMut::merchant_remove_offer",
            "merchant_remove_offer",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Villager.setVillagerData` with a new profession: a different one
    /// forgets the offers, which roll again on the next trade. Clients see the
    /// new profession through entity data. False for an unknown id, a wandering
    /// trader, or a villager with a trading screen open.
    fn merchant_set_profession(
        &mut self,
        merchant: MerchantRef,
        profession: &str,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::merchant_set_profession",
            "merchant_set_profession",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `VillagerData.withLevel`: `Math.max(1, level)`, with no upper limit in
    /// vanilla. The offers stay as they are; the next level-up adds its trades
    /// (a level past 5 can't level up). A trading guest's session follows,
    /// and gets the sequences a level-up from the new level needs.
    fn merchant_set_level(&mut self, merchant: MerchantRef, level: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::merchant_set_level",
            "merchant_set_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `VillagerData.withType`: also the type a trading guest's session
    /// rolls its next level-up offers with. False for an unknown id, a
    /// wandering trader, or a trade open where this can't reach.
    fn merchant_set_villager_type(&mut self, merchant: MerchantRef, kind: &str) -> bool {
        crate::phase::unprovided(
            "BackendMut::merchant_set_villager_type",
            "merchant_set_villager_type",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Villager.setVillagerXp`: assigned as given, as vanilla does.
    fn merchant_set_xp(&mut self, merchant: MerchantRef, xp: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::merchant_set_xp",
            "merchant_set_xp",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
