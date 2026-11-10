# Observing refused damage attempts

`entity_invulnerability_check` observes an attempted server hit without changing its amount.
It supplies the entity, damage source, original invulnerability decision and a writable decision.
A listener that only plays a sound leaves the decision unchanged. There is no amount: vanilla's
`isInvulnerableTo` takes the level and damage source, not damage. Amount mutation belongs on
`living_incoming_damage`, which runs after the early living guards but before the hurt-cooldown
comparison.

For player hits, the observation is delivered once across the server-player, player and living
predicates and the paused equipment continuations. The base decision is retained for that attempt;
the surrounding predicates still apply their own PvP, ability, gamerule, death and client-loaded
guards. A new hit, including one refused by cooldown, starts a new observation. Creative and
PvP-refused player wrappers do not suppress it. Ordinary mob hits also deliver it before death,
fire resistance, shield and cooldown refusal. Ghast and warden overrides deliver their complete
invulnerability decision, including flagged ghasts, reflected fireballs and digging wardens.

A damage-generating precondition is not itself an invulnerability query. In particular:

- Poison at health at or below one does not call vanilla `hurtServer` and produces no observation.
- Spectators are absent from vanilla's explosion entity query; team damage restrictions can also
  prevent an explosion damage call. Neither generates an observation. Creative players remain in
  the query and their attempted explosion damage is observed before ability refusal.
- A sulfur cube with a body item can consume body-immune damage in its `hurtServer` override,
  performing its special push and returning without entering the living invulnerability predicate.
  That path produces no observation, matching the absence of a vanilla predicate call. Ordinary
  sulfur cube damage that enters the living pipeline is observed.
- Client-side prediction, missing targets and unknown resident damage-source identifiers do not
  create authoritative server hit attempts.

The remaining selected non-living paths (armor stands, items, vehicles and experience orbs) retain
the coverage stated in the hook's reference documentation. A separate explicit invulnerability
query is a query, not an additional hit attempt.

The native regression test `native_hit_attempt_tests` counts actual listener notifications on
accepted and refused player and mob routes, equipment continuations, effects, explosions, and
small-fireball, wind-charge and shulker-bullet projectile entries. Unchanged listeners preserve the expected
health, refused fireballs restore prior fire, and only successful shulker hits grant levitation.
A decision-changing case checks that the modified answer survives continuations and a new attempt
asks again. The test uses prepared resources.

Coverage is not yet universal: caller prefilters remain for creative wither-skull and firework
victims and spectator `/damage` targets. These are implementation gaps tracked by #2752, not
vanilla non-attempt exclusions.
