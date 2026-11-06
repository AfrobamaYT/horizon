<!-- chapter: Testing a trait mod -->
<!-- separator: 2 -->

### Testing a trait mod

Every `Host` method defaults to a panic naming it, so a test double implements only what its hook calls, and an
unexpected call fails loudly. Enable the `testing` feature on the mod's `horizon-mod-api` test dependency. Then
`testing::with_mods` takes an owned `Mods` list and runs a closure with it installed on the current thread.
It restores the previous list even if the closure panics. The test allocation is retained because dispatch exposes
a static slice. Call `dispatch_<hook>(&mut host, args…)` inside the closure to build the event as the game does:

```rust
use mod_api::{self, Hooks, Host, HostSide, ServerHost, ModEntry, TickEvent, dispatch_tick, testing::with_mods};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

struct HostDouble;
impl Host for HostDouble {}
impl ServerHost for HostDouble {}
#[derive(Default)]
struct Counter(Arc<AtomicUsize>);
#[mod_api::horizon_mod]
impl Hooks for Counter {
    fn tick(&self, _: HostSide<'_>, _: &mut TickEvent) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}
let count = Arc::new(AtomicUsize::new(0));
let mods = [ModEntry::new("counter", Box::new(Counter(count.clone())))];
with_mods(mods.into(), || dispatch_tick(HostSide::Server(&mut HostDouble)));
assert_eq!(count.load(Ordering::Relaxed), 1);
```

If a `Host` method returns a game value a test can't build, a double can return `None` instead, as with
`Players::acting_player() -> Option<PlayerRef>`. If the dispatch function itself takes an unconstructible value,
such as `dispatch_perform_command`'s `&NativeServerPlayer`, test the logic beneath the hook instead. The level
hosts are small enough to implement whole. NeoForge's GameTest is the in-game counterpart.
