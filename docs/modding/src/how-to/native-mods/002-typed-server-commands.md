<!-- chapter: Typed server commands -->

## Typed server commands

Register each server usage with `register_commands` and `NativeCommand`. The
`perform_command` and `perform_console_command` events carry a `CommandInvocation`,
not a raw line to split. Match its `literals` to your command path, then read named
values through `command.argument(name)` and `CommandArgumentValue`. An integer is
already an `i32`; a player argument is a live `PlayerRef`, not selector text.
Return `Flow::Cancel` when your listener handles the command.

The server parses and checks permissions before resolving arguments or calling a
listener. Omitted optional arguments are absent, including an omitted optional
player: they do not silently select the source player. A present player argument
must select exactly one online player; missing players or invalid input produce
command feedback without calling the listener. Word and greedy text arguments
are strings, numbers are finite doubles, booleans are typed, and item arguments
are component-bearing stacks of one item. Block positions are resolved and floored
from the source's world, relative or local coordinates without loading chunks.
A player's source is their position and rotation. The console has no source
entity and uses the Overworld spawn's block corner with zero rotation, so `@s`
cannot resolve there. Both hooks receive the same owned data, on native and wasm.

