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
