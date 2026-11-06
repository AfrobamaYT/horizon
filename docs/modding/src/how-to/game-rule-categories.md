# Register game rule categories

In `Hooks::register`, call `registrar.game_rule_category("machines")` to declare a category in your mod's namespace. A full ID in your own namespace is also accepted. Invalid IDs, another mod's namespace and duplicate declarations refuse mod loading, even if the returned error is ignored.

The category's translation key is `gamerule.category.<namespace>.<path>`. Categories appear in the host's game rules screen, including when empty. Remote clients do not receive categories in this version. Rule declarations and typed rule access are planned separately; this method declares categories only.
