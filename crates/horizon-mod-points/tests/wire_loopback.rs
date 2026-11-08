#![cfg(feature = "wasm-guest")]

use mod_api::wire::{Reader, Record, Writer};
use mod_points::{
    __private::{Forward, PointHost, SlotListener},
    PointId,
};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Quiet;
impl mod_api::Host for Quiet {}
mod_api::impl_host_areas!(Quiet);
impl mod_api::ServerHost for Quiet {}
mod_api::impl_server_host_areas!(Quiet);
#[mod_api::__subscriptions]
impl mod_api::Hooks for Quiet {}

#[unsafe(no_mangle)]
pub fn __horizon_mod_hooks() -> Box<dyn mod_api::Hooks> {
    Box::new(Quiet)
}

static HEAD: AtomicUsize = AtomicUsize::new(0);
static RETURN: AtomicUsize = AtomicUsize::new(0);
fn head(_: mod_api::HostSide<'_>, args: &mut mod_points::TickArgs) {
    HEAD.fetch_add(1, Ordering::Relaxed);
    args.cancel = true;
}
fn tail(_: mod_api::HostSide<'_>, args: &mut mod_points::TickArgs) {
    RETURN.fetch_add(1, Ordering::Relaxed);
    args.cancel = false;
}

struct Loopback(u32);
impl Forward for Loopback {
    fn forward(&self, id: u32, host: PointHost<'_>, record: &mut dyn Record) {
        let mut input = Writer::new();
        input.put(&self.0);
        input.put(&matches!(
            host,
            PointHost::Side(mod_api::HostSide::Server(_))
        ));
        record.encode(&mut input);
        let pointer = mod_api::wire::guest::input(input.bytes().len());
        // SAFETY: input() reserves this many writable bytes until run().
        unsafe {
            std::ptr::copy_nonoverlapping(input.bytes().as_ptr(), pointer, input.bytes().len());
        }
        let (pointer, len) = mod_api::wire::guest::run(id);
        assert_ne!(len, u32::MAX);
        // SAFETY: run() retains this reply until the next call at this depth.
        let reply = unsafe { std::slice::from_raw_parts(pointer, len as usize) };
        let mut reply = Reader::new(reply);
        reply.take::<()>().unwrap();
        record.apply(reply.rest()).unwrap();
    }
}
static LOOPBACK_HEAD: Loopback = Loopback(0);
static LOOPBACK_RETURN: Loopback = Loopback(1);

#[test]
fn head_and_return_share_the_hook_arena_and_native_listener_list() {
    mod_points::guest::register(&mod_points::listeners::tick("guest", 0, "head", head)).unwrap();
    mod_points::guest::register(&mod_points::listeners::tick_return(
        "guest", 0, "return", tail,
    ))
    .unwrap();
    assert_eq!(
        mod_points::guest::subscriptions(),
        [
            (PointId::Tick as u32, "head".into()),
            (PointId::TickReturn as u32, "return".into())
        ]
    );
    for (place, endpoint) in [
        (PointId::Tick, &LOOPBACK_HEAD),
        (PointId::TickReturn, &LOOPBACK_RETURN),
    ] {
        mod_points::register(&SlotListener::forwarded(
            "guest",
            place,
            endpoint,
            0x8000_0000 + place as u32,
        ))
        .unwrap();
    }
    mod_points::__private::seal();
    let mut args = mod_points::TickArgs::__new();
    mod_points::dispatch::tick(mod_api::HostSide::Server(&mut Quiet), &mut args);
    assert!(args.cancel);
    mod_points::dispatch::tick_return(mod_api::HostSide::Server(&mut Quiet), &mut args);
    assert!(!args.cancel);
    assert_eq!(
        (HEAD.load(Ordering::Relaxed), RETURN.load(Ordering::Relaxed)),
        (1, 1)
    );
    mod_points::stop("guest");
    mod_points::dispatch::tick(mod_api::HostSide::Server(&mut Quiet), &mut args);
    assert!(!args.cancel);
    assert_eq!(HEAD.load(Ordering::Relaxed), 1);
}

#[test]
fn malformed_writeback_is_atomic() {
    let mut args = mod_points::TickArgs::__new();
    assert!(
        args.apply(&[1, 0]).is_err(),
        "trailing bytes are not a partial update"
    );
    assert!(!args.cancel);
    assert!(args.apply(&[]).is_err());
    assert!(!args.cancel);
    args.apply(&[1]).unwrap();
    assert!(args.cancel);
}
