//! The API slot and internals source callbacks share one unified manifest.
mod count;
pub use count::count;
#[derive(Default)]
struct Check;

pub fn tick(_: mod_api::HostSide<'_>, _: &mut mod_points::TickArgs) {}

#[mod_api::horizon_mod]
impl mod_api::Hooks for Check {}
