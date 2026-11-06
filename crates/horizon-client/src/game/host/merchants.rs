//! The game's side of the `merchants` area (`horizon-mod-api/src/host/merchants.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl MerchantsServerHost for Game<'_> {
    fn merchant(&self, entity: EntityRef) -> Option<MerchantRef> {
        mod_api::__private::check_access("merchant", mod_api::__private::LoadAccess::WorldOnly);
        self.native_merchant(entity)
    }
}
impl MerchantsBackend for Game<'_> {
    fn merchant_offers(&self, merchant: MerchantRef) -> Option<Vec<MerchantOffer>> {
        mod_api::__private::check_access(
            "merchant_offers",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_merchant_offers(merchant)
    }
    fn merchant_trading_player(&self, merchant: MerchantRef) -> Option<PlayerRef> {
        mod_api::__private::check_access(
            "merchant_trading_player",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_merchant_trading_player(merchant)
    }
    fn merchant_profession(&self, merchant: MerchantRef) -> Option<String> {
        mod_api::__private::check_access(
            "merchant_profession",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_villager_profession(merchant)
    }
    fn merchant_level(&self, merchant: MerchantRef) -> Option<i32> {
        mod_api::__private::check_access(
            "merchant_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_villager_level(merchant)
    }
    fn merchant_villager_type(&self, merchant: MerchantRef) -> Option<String> {
        mod_api::__private::check_access(
            "merchant_villager_type",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_villager_type(merchant)
    }
    fn merchant_xp(&self, merchant: MerchantRef) -> Option<i32> {
        mod_api::__private::check_access("merchant_xp", mod_api::__private::LoadAccess::WorldOnly);
        self.native_villager_xp(merchant)
    }
}
impl MerchantsBackendMut for Game<'_> {
    fn merchant_set_offer(
        &mut self,
        merchant: MerchantRef,
        index: u32,
        offer: MerchantOffer,
    ) -> bool {
        mod_api::__private::check_access(
            "merchant_set_offer",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_merchant_set_offer(merchant, index, offer)
    }
    fn merchant_add_offer(&mut self, merchant: MerchantRef, offer: MerchantOffer) -> bool {
        mod_api::__private::check_access(
            "merchant_add_offer",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_merchant_add_offer(merchant, offer)
    }
    fn merchant_remove_offer(&mut self, merchant: MerchantRef, index: u32) -> bool {
        mod_api::__private::check_access(
            "merchant_remove_offer",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_merchant_remove_offer(merchant, index)
    }
    fn merchant_set_profession(&mut self, merchant: MerchantRef, profession: &str) -> bool {
        mod_api::__private::check_access(
            "merchant_set_profession",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_villager_set_profession(merchant, profession)
    }
    fn merchant_set_level(&mut self, merchant: MerchantRef, level: i32) -> bool {
        mod_api::__private::check_access(
            "merchant_set_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_villager_set_level(merchant, level)
    }
    fn merchant_set_villager_type(&mut self, merchant: MerchantRef, kind: &str) -> bool {
        mod_api::__private::check_access(
            "merchant_set_villager_type",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_villager_set_type(merchant, kind)
    }
    fn merchant_set_xp(&mut self, merchant: MerchantRef, xp: i32) -> bool {
        mod_api::__private::check_access(
            "merchant_set_xp",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_villager_set_xp(merchant, xp)
    }
}
