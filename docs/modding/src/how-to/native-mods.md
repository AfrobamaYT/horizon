# Native mods

<!-- concept-pages -->

## Level sounds

Implement `play_level_sound_at_position` or `play_level_sound_at_entity` to edit
or cancel a level sound before it is broadcast (server) or played (client).
Match the hook's `HostSide` to distinguish the requests; the event's sound,
source, volume and pitch describe that side's request, and original volume and
pitch remain available after edits. Returning `Flow::Cancel` suppresses that
request. A server cancellation sends no packet, so there is no client request.

Incoming sound packets, including `/playsound`, ask the client level hooks.
An entity-sound packet stays bound to its emitter. UI clicks, engine-local
sounds and `ClientHost::play_local_sound` do not ask these level hooks; the
sound engine's `play_sound` hook remains a separate, later interception point.

Client-predicted level sounds and `LocalPlayer.playSound` use those same hooks.
Canceling their sound does not cancel the player action or undo a predicted
block change. A local player's ordinary sound is an `AtEntity` request but
still plays at the requested position; only a genuinely entity-bound sound
(such as a goat horn) follows its emitter after playback starts.

| Client route | Level event | Vanilla call being represented |
| --- | --- | --- |
| Placement confirmation, predicted doors/buttons/levers | AtPosition | `BlockItem.place`, block `useWithoutItem` → `Level.playSound` |
| Waxing, tool transforms, bucket use | AtPosition | `HoneycombItem.useOn`, tool `useOn`, `BucketItem` → `Level.playSound` |
| Own steps, swimming, splash, amethyst chime, honey slide, fall | AtEntity | `Entity`/`LivingEntity` → virtual `LocalPlayer.playSound` |
| Own hurt/death, bundle menu clicks, consume effects | AtEntity | `LivingEntity`, `BundleItem`, `Consumable` → `LocalPlayer.playSound` |
| Player-bound sounds, including the local goat horn | AtEntity | `Level.playSound(except, entity, ...)` |
| Block-breaking/level-event effects, explosions, pickup packets | None | Client-level `playLocalSound` or direct sound manager playback |
| Dragon/fang/sniffer local effects, ticking ambience, UI | None | Entity-local effects, sound instances or widgets, not `playSeededSound` |

Server-only gameplay sounds keep their existing broadcast-time event. They are
not moved earlier merely because their implementation also calls `Audio`.
