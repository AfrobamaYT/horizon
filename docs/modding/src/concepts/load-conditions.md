# Registered data load conditions

Implement `LoadCondition`, then register it from
`Hooks::register_load_conditions` using its `RegistrationContext` and
`event.conditions.register(Identifier::parse("testmod:flag").unwrap(), Flag)`.
Registration happens once per mod load. The name must belong to the calling
mod's namespace. Duplicate names refuse registration and name both owners;
ignoring a registration error still refuses admission.

```rust,ignore
struct Flag;
impl mod_api::LoadCondition for Flag {
    fn test(&self, args: &mod_api::serde_json::Value)
        -> Result<bool, mod_api::ConditionError>
    {
        args["value"].as_bool().ok_or_else(|| mod_api::ConditionError {
            message: "value must be a boolean".into(),
        })
    }
}
```

A recipe can use the registered name alongside the built-in conditions:

```json
{
  "type": "minecraft:crafting_shapeless",
  "ingredients": ["minecraft:flint"],
  "result": {"id": "minecraft:stick"},
  "neoforge:conditions": [{"type": "testmod:flag", "value": true}]
}
```

The handler receives the JSON object without `type`, retaining JSON booleans,
numbers, strings, lists and objects. `ConditionError` has an explicit Wire
codec. Native and WebAssembly registrants cross the same generated route.
Handlers run in the server's candidate data reload, before the entries become
visible. Results are retained for each entry and condition payload in that
candidate, so repeated readers do not call the handler again. A new reload
reevaluates the entry. Handlers must not mutate live data. A callback panic or
wasm trap follows the runtime's normal failure policy.

`true` loads the entry; `false` omits it, without exposing a lower pack's
version. A callback error refuses the reload and names the file. The previous
usable recipe and resource generation stays installed.

A namespace is owned only when a loaded mod registered at least one condition
in it. A loaded mod ID alone does not own a condition namespace. An unknown
name in an owned namespace refuses reload, naming the file, condition and
registering mod. This also applies at every depth inside `not`, `and` and `or`,
even beside an already false or true branch.

An unknown name in any other namespace keeps the existing fail-closed behavior:
the file is absent, including beneath the combinators. One warning per file
names the file and its distinct unknown condition names. Existing built-in
conditions and files without conditions continue to use the same evaluator.

The public [NeoForge data load condition documentation](https://docs.neoforged.net/docs/resources/server/conditions/)
describes `ICondition` and the top-level `neoforge:conditions` array. Horizon's
owned-namespace refusal and foreign-name warning rule is its own admission
contract; it does not claim NeoForge error-policy parity.
