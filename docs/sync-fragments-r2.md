# Incoming fragments at 501446443c

## living_death_message — converted

Converted with tools/convert_sync_fragment.py to the actual catalog location,
crates/horizon-mod-vocab/src/hooks (horizon-mod-api/src/hooks does not exist).
A server collect hook edits Option<Text> through the generated ServerHost dispatcher;
no new public value type or Wire implementation is needed. LivingDeathEvent and
LivingDeathPostEvent describe different moments, not the owner message. The game
fires after the owner-online and show-death-messages guards. Removed the unused
incoming library wrapper calling a nonexistent native_client_hooks dispatcher.
No runtime claim; CI must validate generated dispatch.

## line_of_sight

Refused as a hook fragment: it has no hook moments, only read helpers.
Existing typed LivingRef::has_line_of_sight (host/living.rs) and LevelRef::clip
(host/levels.rs, collider and no-fluid choices) own these queries. Upstream's
native_entity_refs.rs clips eye-to-eye within the current level or clips an
empty-context segment. Do not add native_ aliases or a second notification.

Converter boundary: `unsupported fragment fields: ['comments', 'doc_summary', 'helpers', 'rust_summary'] or draw contexts; helpers are host methods now, written by hand in crates/horizon-mod-vocab/src/host/<area>.rs`.
