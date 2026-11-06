# Declaring a machine's menu data

A mod block entity has two different kinds of storage: its item slots are reached
through transactional Parts; its integer menu data is read with
`BlockEntityRef::data_slots` and changed on a server Host. Menu data is transient:
creation and loading both start with an empty data vector, even when the saved
payload still contains the machine's progress.

## Size the data vector

Call `entity.set_data_slots(host, initial_values)` with the complete initial
vector when preparing a newly created or loaded machine. For a progress display,
use one element initialized from the progress value in its saved payload. This is
the live-handle replacement for assigning a mutable tick payload's `data` field.
Do not reset the vector to its initial values on every tick.

The method accepts at most the game's mod-menu data-slot limit (currently 256).
It returns `Ok(true)` when it changes the vector, `Ok(false)` for identical values,
or `BlockEntityDataError::TooManySlots` with the submitted count and limit. It
never silently clamps a vector. `BlockEntityDataError::Unavailable` means the
handle cannot reach a writable mod holder in the server level currently in place.

## Update individual values

After sizing, call `entity.set_data_slot(host, index, value)` for scalar updates.
This method does not resize: an absent index returns false. Both methods use the
same write target, journal and `setChanged` path. Nested transactions retain their
ordinary behavior; an aborted outer transaction restores the vector that existed
before its first write, including writes committed by children. The normal menu
broadcast delivers committed changes to viewers.

The public handle still resolves a level and position. Removal makes it
unavailable; replacement at that position is followed, as with the other handle
methods. This is not an instance-token or borrowed-payload API.
