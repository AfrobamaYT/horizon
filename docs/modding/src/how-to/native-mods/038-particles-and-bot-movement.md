<!-- chapter: Client particles and movement queries -->
<!-- separator: 0 -->

### Client particles

`ClientHost::add_particle` submits the API's `ParticleOptions`, a world position and
velocity to the ordinary client particle queue. The client must have a loaded player
and world. Non-finite numeric inputs are refused. A successful submission does not
promise a visible particle: provider availability, sprite sets, distance and the
player's particle settings still apply. This method is not available on `ServerHost`.

### Bot movement and block collisions

`PlayerRef::simulate_move` predicts one server player's movement tick from
`NativePlayerInput`, using the game's movement implementation without replacing the
live player's body. It returns `NativeSimulatedMove` with position, velocity, ground,
horizontal-collision and sprinting results. It refuses stale or inaccessible players,
dead players, spectators, riders and invalid numeric input. A supplied velocity is
input to the prediction, not a write to the player.

`LevelRef::collision_boxes` reads block collision boxes within a finite, ordered
axis-aligned box. It excludes entity and border collisions. A refused level or a
query covering more than 1,000,000 cells returns `None`; a valid query with no
collisions returns `Some` containing an empty vector. Keep queries bounded rather
than scanning a whole level each tick.

Both queries take typed handles for their natural receiver. There are no parallel
UUID-only or dimension-string helpers.
