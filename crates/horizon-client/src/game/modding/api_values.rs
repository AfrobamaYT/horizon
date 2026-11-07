//! The game-owned conversion boundary between API records and engine values.
//! Rust's orphan rule prevents foreign From/TryFrom implementations here.
use horizon_core::nbt::{JavaString, Tag};
use horizon_items::items::{ItemStack as EngineStack, Items};
use mod_api::item_stack::{ItemStack as ApiStack, StackWriteError};

/// Convert an engine value into an owned API value after resolving registry identities.
pub(in crate::game) trait TryIntoApi {
    type Api;
    type Error;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error>;
}

/// Validate a mod value against the game's registry before admitting it to engine state.
pub(in crate::game) trait TryIntoEngine {
    type Engine;
    type Error;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::game) enum StackReadError {
    UnknownItem,
    Empty,
}

impl TryIntoApi for EngineStack {
    type Api = ApiStack;
    type Error = StackReadError;

    fn try_into_api(&self) -> Result<ApiStack, Self::Error> {
        let key = mod_api::__private::item_key(&self.item).ok_or(StackReadError::UnknownItem)?;
        let mut result = ApiStack::new(key, self.count).ok_or(StackReadError::Empty)?;
        result.set_damage(self.damage);
        if let Some(Tag::Compound(mut patch)) = horizon_items::item_data::encode_components(self) {
            patch.remove(&JavaString::from("minecraft:damage"));
            result.set_components(patch);
        }
        Ok(result)
    }
}

impl TryIntoEngine for ApiStack {
    type Engine = EngineStack;
    type Error = StackWriteError;

    fn try_into_engine(&self, items: &Items) -> Result<EngineStack, Self::Error> {
        let id =
            mod_api::__private::item_key_id(self.item()).ok_or(StackWriteError::UnknownItem)?;
        let (id, definition) = items
            .items
            .get_key_value(id)
            .ok_or(StackWriteError::UnknownItem)?;
        let patch = self
            .components()
            .map(|patch| Tag::Compound(patch.as_nbt().clone()));
        let (_, extra) = horizon_items::item_data::decode_components(patch.as_ref());
        let stack = EngineStack {
            item: id.clone(),
            count: self.count(),
            damage: self.damage(),
            extra,
        };
        let effective = |name: &str, default: u32| -> Result<u32, StackWriteError> {
            match stack.data().and_then(|data| data.other(name)) {
                Some(Tag::Int(value)) => {
                    u32::try_from(*value).map_err(|_| StackWriteError::InvalidMaximum)
                }
                Some(_) => Err(StackWriteError::InvalidMaximum),
                None => Ok(default),
            }
        };
        let maximum = effective("minecraft:max_stack_size", definition.max_stack)?;
        if stack.count > maximum {
            return Err(StackWriteError::CountExceeded { maximum });
        }
        let maximum = effective("minecraft:max_damage", definition.max_damage)?;
        if stack.damage > maximum {
            return Err(StackWriteError::DamageExceeded { maximum });
        }
        Ok(stack)
    }
}

impl TryIntoApi for crate::crafting::RecipeRule {
 type Api = mod_api::RecipeRule;
 type Error = StackReadError;
 fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
 Ok(match self {
Self::Stonecutting { input, result } => mod_api::RecipeRule::Stonecutting {
input: input.try_into_api()?,
result: result.try_into_api()?,
},
Self::SmithingTransform { template, base, addition, result } => mod_api::RecipeRule::SmithingTransform {
template: template.try_into_api()?,
base: base.try_into_api()?,
addition: addition.try_into_api()?,
result: result.try_into_api()?,
},
Self::SmithingTrim { template, base, addition, pattern } => mod_api::RecipeRule::SmithingTrim {
template: template.try_into_api()?,
base: base.try_into_api()?,
addition: addition.try_into_api()?,
pattern: pattern.try_into_api()?,
},
Self::BannerDuplicate { banner, result } => mod_api::RecipeRule::BannerDuplicate {
banner: banner.try_into_api()?,
result: result.try_into_api()?,
},
Self::ShieldDecoration { banner, target, result } => mod_api::RecipeRule::ShieldDecoration {
banner: banner.try_into_api()?,
target: target.try_into_api()?,
result: result.try_into_api()?,
},
Self::Transmute { input, material, min, max, add_count, result } => mod_api::RecipeRule::Transmute {
input: input.try_into_api()?,
material: material.try_into_api()?,
min: *min,
max: *max,
add_count: *add_count,
result: result.try_into_api()?,
},
Self::Dye { target, dye, result } => mod_api::RecipeRule::Dye {
target: target.try_into_api()?,
dye: dye.try_into_api()?,
result: result.try_into_api()?,
},
Self::Imbue { source, material, result } => mod_api::RecipeRule::Imbue {
source: source.try_into_api()?,
material: material.try_into_api()?,
result: result.try_into_api()?,
},
Self::BookCloning { source, material, min, max, result } => mod_api::RecipeRule::BookCloning {
source: source.try_into_api()?,
material: material.try_into_api()?,
min: *min,
max: *max,
result: result.try_into_api()?,
},
Self::FireworkRocket { shell, fuel, star, result } => mod_api::RecipeRule::FireworkRocket {
shell: shell.try_into_api()?,
fuel: fuel.try_into_api()?,
star: star.try_into_api()?,
result: result.try_into_api()?,
},
Self::FireworkStar { dye, fuel, trail, twinkle, shapes, result } => mod_api::RecipeRule::FireworkStar {
dye: dye.try_into_api()?,
fuel: fuel.try_into_api()?,
trail: trail.try_into_api()?,
twinkle: twinkle.try_into_api()?,
shapes: shapes.try_into_api()?,
result: result.try_into_api()?,
},
Self::FireworkStarFade { dye, target, result } => mod_api::RecipeRule::FireworkStarFade {
dye: dye.try_into_api()?,
target: target.try_into_api()?,
result: result.try_into_api()?,
},
Self::MapExtending { map, material, result } => mod_api::RecipeRule::MapExtending {
map: map.try_into_api()?,
material: material.try_into_api()?,
result: result.try_into_api()?,
},
Self::DecoratedPot { back, left, right, front, result } => mod_api::RecipeRule::DecoratedPot {
back: back.try_into_api()?,
left: left.try_into_api()?,
right: right.try_into_api()?,
front: front.try_into_api()?,
result: result.try_into_api()?,
},
Self::RepairItem => mod_api::RecipeRule::RepairItem,
})
 }
}

impl<T: TryIntoApi<Error = StackReadError>> TryIntoApi for Option<T> {
 type Api = Option<T::Api>; type Error = StackReadError;
 fn try_into_api(&self) -> Result<Self::Api, Self::Error> { self.as_ref().map(TryIntoApi::try_into_api).transpose() }
}
impl<T: TryIntoApi<Error = StackReadError>> TryIntoApi for Vec<T> {
 type Api = Vec<T::Api>; type Error = StackReadError;
 fn try_into_api(&self) -> Result<Self::Api, Self::Error> { self.iter().map(TryIntoApi::try_into_api).collect() }
}
impl TryIntoApi for crate::crafting::Ingredient {
 type Api = mod_api::Ingredient; type Error = StackReadError;
 fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
 Ok(mod_api::Ingredient { items: self.items.clone(), order: self.order.clone(), custom: self.custom.clone(), children: self.children.as_ref().map(|(shape,kids)| Ok((shape.clone(), kids.try_into_api()?))).transpose()? })
 }
}
impl TryIntoEngine for mod_api::Ingredient {
 type Engine = crate::crafting::Ingredient; type Error = StackWriteError;
 fn try_into_engine(&self, items: &Items) -> Result<Self::Engine,Self::Error> {
 Ok(crate::crafting::Ingredient { items: self.items.clone(), order: self.order.clone(), custom: self.custom.clone(), children: self.children.as_ref().map(|(shape,kids)| Ok((shape.clone(), kids.iter().map(|k| k.try_into_engine(items)).collect::<Result<_,_>>()?))).transpose()? })
 }
}
impl TryIntoApi for crate::crafting::RecipeTemplate {
 type Api = mod_api::RecipeTemplate; type Error = StackReadError;
 fn try_into_api(&self) -> Result<Self::Api,Self::Error> { Ok(mod_api::RecipeTemplate::new(self.item.clone(),self.count,self.components.clone())) }
}
impl TryIntoApi for crate::crafting::RecipeTrimPattern {
 type Api = mod_api::RecipeTrimPattern; type Error = StackReadError;
 fn try_into_api(&self) -> Result<Self::Api,Self::Error> { Ok(match self { Self::Reference(id) => mod_api::RecipeTrimPattern::Reference(id.clone()), Self::Inline { asset_id,description,decal } => mod_api::RecipeTrimPattern::Inline { asset_id:asset_id.clone(),description:description.clone(),decal:*decal } }) }
}
impl TryIntoApi for crate::crafting::FireworkShapeIngredient {
 type Api = mod_api::FireworkShapeIngredient; type Error = StackReadError;
 fn try_into_api(&self) -> Result<Self::Api,Self::Error> { Ok(mod_api::FireworkShapeIngredient::new(self.shape,self.ingredient.try_into_api()?)) }
}
impl TryIntoApi for crate::crafting::RecipeSnapshot {
 type Api = mod_api::Recipe; type Error = StackReadError;
 fn try_into_api(&self) -> Result<Self::Api,Self::Error> {
 let mut result = mod_api::Recipe::new(self.id.clone(), self.kind.clone(),self.json.clone());
 result.group=self.group.clone(); result.ingredients=self.ingredients.try_into_api()?; result.results=self.results.try_into_api()?; result.cooking_time=self.cooking_time; result.experience=self.experience; result.rule=self.rule.try_into_api()?; Ok(result)
 }
}
