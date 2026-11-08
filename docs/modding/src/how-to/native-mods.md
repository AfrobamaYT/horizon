# Native mods

<!-- concept-pages -->

## Refusing dimension travel

`entity_travel_to_dimension` runs before a server entity moves to the target
level. Returning `Flow::Cancel` refuses that entity's travel. Passengers are
asked recursively: refusing one leaves it dismounted in the source level,
with its own passengers, without refusing the vehicle's transition. Discarding
an entity through its handle during the callback also prevents its travel.
A falling block ticks outside the level's entity list, so its handle is not
accessible during this callback.
