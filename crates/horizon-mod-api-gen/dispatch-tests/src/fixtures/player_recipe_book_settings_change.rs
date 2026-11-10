mod api {
    pub(super) enum RecipeBookIdentity {
        Crafting,
        Furnace,
        BlastFurnace,
        Smoker,
        Custom(String),
    }
}
