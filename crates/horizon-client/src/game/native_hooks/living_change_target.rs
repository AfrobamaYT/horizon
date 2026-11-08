use super::*;

impl Game<'_> {
    /// A mob is about to acquire or clear a target.
    pub(in crate::game) fn native_living_change_target(
        &self,
        call: mod_api::__private::LivingChangeTargetCall<'_>,
    ) {
        self.fire_living_change_target(call);
    }
}
