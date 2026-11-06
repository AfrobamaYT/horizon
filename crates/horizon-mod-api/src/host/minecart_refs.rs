//! The `minecart_refs` area: host methods the game implements (`game/host/minecart_refs.rs`).
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `minecart_refs` area, implemented by the game and reached through [`ServerHost`].
pub trait MinecartRefsServerHost {
    /// Side: server.
    /// Narrows an entity to a minecart of any kind, or None for another kind of
    /// entity, a stale handle, a client, or an entity held out of storage.
    fn minecart(&self, entity: EntityRef) -> Option<MinecartRef> {
        crate::phase::unprovided(
            "ServerHost::minecart",
            "minecart",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `minecart_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait MinecartRefsBackend {
    /// Side: server.
    /// Which `AbstractMinecart` subclass the cart is.
    fn minecart_kind(&self, cart: MinecartRef) -> Option<NativeMinecartKind> {
        crate::phase::unprovided(
            "Backend::minecart_kind",
            "minecart_kind",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractMinecart.getMaxSpeed`, in blocks per tick: the old behaviour's
    /// 0.4 (0.2 in water), or with the `minecart_improvements` experiment the
    /// `max_minecart_speed` rule over 20 (halved in water). A furnace minecart
    /// takes half of that, three quarters in water (`MinecartFurnace`).
    fn minecart_max_speed(&self, cart: MinecartRef) -> Option<f64> {
        crate::phase::unprovided(
            "Backend::minecart_max_speed",
            "minecart_max_speed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractMinecart.isOnRails`.
    fn minecart_is_on_rails(&self, cart: MinecartRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::minecart_is_on_rails",
            "minecart_is_on_rails",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractMinecart.isFlipped`.
    fn minecart_flipped(&self, cart: MinecartRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::minecart_flipped",
            "minecart_flipped",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `VehicleEntity.getDamage`, the wobble damage (`DATA_ID_DAMAGE`).
    fn minecart_damage(&self, cart: MinecartRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::minecart_damage",
            "minecart_damage",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `VehicleEntity.getHurtTime`, the wobble ticks (`DATA_ID_HURT`).
    fn minecart_hurt_time(&self, cart: MinecartRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::minecart_hurt_time",
            "minecart_hurt_time",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractMinecart.customDisplayBlockState`: the block the cart shows in
    /// place of its kind's default, None (inner) when it has none.
    fn minecart_custom_display_block(
        &self,
        cart: MinecartRef,
    ) -> Option<Option<BlockStateId>> {
        crate::phase::unprovided(
            "Backend::minecart_custom_display_block",
            "minecart_custom_display_block",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractMinecart.getDisplayOffset`, in sixteenths of a block: the set
    /// offset, else the kind's default.
    fn minecart_display_offset(&self, cart: MinecartRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::minecart_display_offset",
            "minecart_display_offset",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `MinecartFurnace`'s fuel ticks; None for another kind of cart.
    fn minecart_fuel(&self, cart: MinecartRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::minecart_fuel",
            "minecart_fuel",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `MinecartTNT`'s fuse ticks, -1 when it is not primed; None for another
    /// kind of cart.
    fn minecart_fuse(&self, cart: MinecartRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::minecart_fuse",
            "minecart_fuse",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `MinecartHopper.isEnabled`; None for another kind of cart.
    fn minecart_hopper_enabled(&self, cart: MinecartRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::minecart_hopper_enabled",
            "minecart_hopper_enabled",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// A command block minecart's command (`MinecartCommandBlock.getCommandBlock().getCommand()`).
    /// None for another kind of cart.
    fn minecart_command(&self, cart: MinecartRef) -> Option<String> {
        crate::phase::unprovided(
            "Backend::minecart_command",
            "minecart_command",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `minecart_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait MinecartRefsBackendMut {
    /// Side: server.
    /// `AbstractMinecart.setFlipped`; saved as `FlippedRotation`, not synced.
    fn minecart_set_flipped(&mut self, cart: MinecartRef, flipped: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::minecart_set_flipped",
            "minecart_set_flipped",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `VehicleEntity.setDamage`: synced to clients. The tick counts it down
    /// only while positive.
    fn minecart_set_damage(&mut self, cart: MinecartRef, damage: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::minecart_set_damage",
            "minecart_set_damage",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `VehicleEntity.setHurtTime`: synced to clients.
    fn minecart_set_hurt_time(&mut self, cart: MinecartRef, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::minecart_set_hurt_time",
            "minecart_set_hurt_time",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the block the cart shows, or with None its kind's default again:
    /// saved as `DisplayState` and synced to clients. False for a state the
    /// registry doesn't know, as the loader drops one that does not decode.
    fn minecart_set_custom_display_block(
        &mut self,
        cart: MinecartRef,
        state: Option<BlockStateId>,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::minecart_set_custom_display_block",
            "minecart_set_custom_display_block",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractMinecart.setDisplayOffset`, in sixteenths of a block: saved as
    /// `DisplayOffset` (only off the kind's default) and synced to clients.
    fn minecart_set_display_offset(&mut self, cart: MinecartRef, offset: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::minecart_set_display_offset",
            "minecart_set_display_offset",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets a furnace minecart's fuel ticks, any int as vanilla's field holds
    /// it (saved as a short, the low 16 bits). False for another kind of cart.
    /// The lit flag (`DATA_ID_FUEL`) follows with fuel above zero.
    fn minecart_set_fuel(&mut self, cart: MinecartRef, fuel: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::minecart_set_fuel",
            "minecart_set_fuel",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `MinecartTNT.primeFuse(null)`: an 80 tick fuse, the primed sound and
    /// entity event 70 for clients. False for another kind of cart, or with
    /// the `tnt_explodes` rule off.
    fn minecart_prime_fuse(&mut self, cart: MinecartRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::minecart_prime_fuse",
            "minecart_prime_fuse",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `MinecartHopper.setEnabled`: saved as `Enabled`, not synced.
    fn minecart_set_hopper_enabled(&mut self, cart: MinecartRef, enabled: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::minecart_set_hopper_enabled",
            "minecart_set_hopper_enabled",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets a command block minecart's command (`BaseCommandBlock.setCommand`, which resets the
    /// success count), then `MinecartCommandBase.onUpdated`, as the edit screen's save does: the
    /// synced command and last output follow, so clients' editors show the new command. The
    /// output and its tracking are kept, and no chat note is sent. An activator rail then runs
    /// the new command. False for another kind of cart.
    fn minecart_set_command(&mut self, cart: MinecartRef, command: &str) -> bool {
        crate::phase::unprovided(
            "BackendMut::minecart_set_command",
            "minecart_set_command",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
