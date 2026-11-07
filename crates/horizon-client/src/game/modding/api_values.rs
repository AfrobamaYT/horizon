// game-module-visibility: pub(crate)
//! The game-owned conversion boundary between API records and engine values.
//! Rust's orphan rule prevents foreign From/TryFrom implementations here.
use horizon_core::nbt::{JavaString, Tag};
use horizon_items::items::{ItemStack as EngineStack, Items};
use mod_api::item_stack::{ItemStack as ApiStack, StackWriteError};

/// Registry access for generated stack write-back in full and narrowed hook hosts.
pub(in crate::game) trait ItemBoundary {
    fn boundary_items(&self) -> Option<&Items>;
}
impl ItemBoundary for super::Game<'_> {
    fn boundary_items(&self) -> Option<&Items> {
        Some(self.items())
    }
}
impl ItemBoundary for super::GameLevel<'_> {
    fn boundary_items(&self) -> Option<&Items> {
        Some(self.items)
    }
}
impl ItemBoundary for super::native_hooks::HitLevel<'_> {
    fn boundary_items(&self) -> Option<&Items> {
        self.0.tags()
    }
}

/// Normalize an existing game write's answer without changing its implementation.
/// Generated API forwarders add the checked-input refusal around successful native answers.
pub(in crate::game) trait IntoWriteResult {
    type Value;
    type Error;
    fn into_write_result(self) -> Result<Self::Value, Self::Error>;
}
impl<T> IntoWriteResult for Option<T> {
    type Value = Self;
    type Error = StackWriteError;
    fn into_write_result(self) -> Result<Self, Self::Error> {
        Ok(self)
    }
}
impl<T, E> IntoWriteResult for Result<T, E> {
    type Value = T;
    type Error = E;
    fn into_write_result(self) -> Self {
        self
    }
}
macro_rules! write_answers {
    ($($ty:ty),* $(,)?) => { $(
        impl IntoWriteResult for $ty {
            type Value = Self;
            type Error = StackWriteError;
            fn into_write_result(self) -> Result<Self, Self::Error> { Ok(self) }
        }
    )* };
}
write_answers!(bool, u32, ());

/// Convert an engine value into an owned API value after resolving registry identities.
pub(in crate::game) trait TryIntoApi {
    type Api;
    type Error;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error>;
}

/// Validate a mod value against the game's registry before admitting it to engine state.
pub(in crate::game) trait TryIntoEngine<Context: ?Sized = Items> {
    type Engine;
    type Error;
    fn try_into_engine(&self, context: &Context) -> Result<Self::Engine, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::game) enum StackReadError {
    UnknownItem,
    Empty,
}

impl StackReadError {
    pub(in crate::game) fn into_write_error(self) -> StackWriteError {
        match self {
            Self::UnknownItem => StackWriteError::UnknownItem,
            Self::Empty => StackWriteError::EmptyEngineStack,
        }
    }
}

macro_rules! scalar_values {
    ($($ty:ty),* $(,)?) => { $(
        impl TryIntoApi for $ty {
            type Api = Self;
            type Error = StackReadError;
            fn try_into_api(&self) -> Result<Self, Self::Error> { Ok(*self) }
        }
        impl TryIntoEngine for $ty {
            type Engine = Self;
            type Error = StackWriteError;
            fn try_into_engine(&self, _: &Items) -> Result<Self, Self::Error> { Ok(*self) }
        }
    )* };
}
scalar_values!(u32, i32, bool, ());

impl<T: TryIntoApi + ?Sized> TryIntoApi for &T {
    type Api = T::Api;
    type Error = T::Error;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        (*self).try_into_api()
    }
}
impl<T: TryIntoEngine + ?Sized> TryIntoEngine for &T {
    type Engine = T::Engine;
    type Error = T::Error;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        (*self).try_into_engine(items)
    }
}

impl<T: TryIntoApi> TryIntoApi for Option<T> {
    type Api = Option<T::Api>;
    type Error = T::Error;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        self.as_ref().map(TryIntoApi::try_into_api).transpose()
    }
}
impl<T: TryIntoEngine> TryIntoEngine for Option<T> {
    type Engine = Option<T::Engine>;
    type Error = T::Error;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        self.as_ref()
            .map(|value| value.try_into_engine(items))
            .transpose()
    }
}
impl<T: TryIntoApi, E: Clone> TryIntoApi for Result<T, E> {
    type Api = Result<T::Api, E>;
    type Error = T::Error;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        match self {
            Ok(value) => value.try_into_api().map(Ok),
            Err(error) => Ok(Err(error.clone())),
        }
    }
}

impl<T: TryIntoApi, const N: usize> TryIntoApi for [T; N] {
    type Api = [T::Api; N];
    type Error = T::Error;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        let mut output: [Option<T::Api>; N] = std::array::from_fn(|_| None);
        for (target, source) in output.iter_mut().zip(self) {
            *target = Some(source.try_into_api()?);
        }
        Ok(output.map(|value| value.expect("every array element converted")))
    }
}
impl<T: TryIntoEngine, const N: usize> TryIntoEngine for [T; N] {
    type Engine = [T::Engine; N];
    type Error = T::Error;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        let mut output: [Option<T::Engine>; N] = std::array::from_fn(|_| None);
        for (target, source) in output.iter_mut().zip(self) {
            *target = Some(source.try_into_engine(items)?);
        }
        Ok(output.map(|value| value.expect("every array element converted")))
    }
}
impl<T: TryIntoApi> TryIntoApi for [T] {
    type Api = Vec<T::Api>;
    type Error = T::Error;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        self.iter().map(TryIntoApi::try_into_api).collect()
    }
}
impl<T: TryIntoApi> TryIntoApi for std::sync::Arc<[T]> {
    type Api = std::sync::Arc<[T::Api]>;
    type Error = T::Error;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(self.as_ref().try_into_api()?.into())
    }
}
impl<T: TryIntoApi> TryIntoApi for Vec<T> {
    type Api = Vec<T::Api>;
    type Error = T::Error;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        self.iter().map(TryIntoApi::try_into_api).collect()
    }
}
impl<T: TryIntoEngine> TryIntoEngine for Vec<T> {
    type Engine = Vec<T::Engine>;
    type Error = T::Error;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        self.as_slice().try_into_engine(items)
    }
}
impl<T: TryIntoEngine> TryIntoEngine for [T] {
    type Engine = Vec<T::Engine>;
    type Error = T::Error;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        self.iter()
            .map(|value| value.try_into_engine(items))
            .collect()
    }
}
impl<A: TryIntoApi, B: TryIntoApi<Error = A::Error>> TryIntoApi for (A, B) {
    type Api = (A::Api, B::Api);
    type Error = A::Error;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok((self.0.try_into_api()?, self.1.try_into_api()?))
    }
}
impl<A: TryIntoEngine, B: TryIntoEngine<Error = A::Error>> TryIntoEngine for (A, B) {
    type Engine = (A::Engine, B::Engine);
    type Error = A::Error;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        Ok((
            self.0.try_into_engine(items)?,
            self.1.try_into_engine(items)?,
        ))
    }
}

impl TryIntoEngine for mod_api::NativeItemDisplayContext {
    type Engine = horizon_assets::item_model::DisplayContext;
    type Error = ();
    fn try_into_engine(&self, _: &Items) -> Result<Self::Engine, Self::Error> {
        use horizon_assets::item_model::DisplayContext as Engine;
        use mod_api::NativeItemDisplayContext as Api;
        Ok(match self {
            Api::None => Engine::None,
            Api::ThirdPersonLeftHand => Engine::ThirdPersonLeftHand,
            Api::ThirdPersonRightHand => Engine::ThirdPersonRightHand,
            Api::FirstPersonLeftHand => Engine::FirstPersonLeftHand,
            Api::FirstPersonRightHand => Engine::FirstPersonRightHand,
            Api::Head => Engine::Head,
            Api::Gui => Engine::Gui,
            Api::Ground => Engine::Ground,
            Api::Fixed => Engine::Fixed,
            Api::OnShelf => Engine::OnShelf,
            _ => return Err(()),
        })
    }
}

impl TryIntoApi for horizon_client::merchant::ItemCost {
    type Api = mod_api::ItemCost;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(mod_api::ItemCost::new(self.stack.try_into_api()?))
    }
}
impl TryIntoEngine for mod_api::ItemCost {
    type Engine = horizon_client::merchant::ItemCost;
    type Error = StackWriteError;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        Ok(Self::Engine {
            stack: self.stack.try_into_engine(items)?,
        })
    }
}
impl TryIntoApi for horizon_client::merchant::MerchantOffer {
    type Api = mod_api::MerchantOffer;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        let mut offer = mod_api::MerchantOffer::new(
            self.base_cost_a.try_into_api()?,
            self.cost_b.try_into_api()?,
            self.result.try_into_api()?,
            self.max_uses,
            self.xp,
            self.price_multiplier,
        );
        offer.uses = self.uses;
        offer.reward_exp = self.reward_exp;
        offer.special_price_diff = self.special_price_diff;
        offer.demand = self.demand;
        Ok(offer)
    }
}
impl TryIntoEngine for mod_api::MerchantOffer {
    type Engine = horizon_client::merchant::MerchantOffer;
    type Error = StackWriteError;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        Ok(Self::Engine {
            base_cost_a: self.base_cost_a.try_into_engine(items)?,
            cost_b: self.cost_b.try_into_engine(items)?,
            result: self.result.try_into_engine(items)?,
            uses: self.uses,
            max_uses: self.max_uses,
            reward_exp: self.reward_exp,
            special_price_diff: self.special_price_diff,
            demand: self.demand,
            price_multiplier: self.price_multiplier,
            xp: self.xp,
        })
    }
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

impl TryIntoEngine for mod_api::ItemResource {
    type Engine = EngineStack;
    type Error = StackWriteError;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        self.as_stack().try_into_engine(items)
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
            Self::SmithingTransform {
                template,
                base,
                addition,
                result,
            } => mod_api::RecipeRule::SmithingTransform {
                template: template.try_into_api()?,
                base: base.try_into_api()?,
                addition: addition.try_into_api()?,
                result: result.try_into_api()?,
            },
            Self::SmithingTrim {
                template,
                base,
                addition,
                pattern,
            } => mod_api::RecipeRule::SmithingTrim {
                template: template.try_into_api()?,
                base: base.try_into_api()?,
                addition: addition.try_into_api()?,
                pattern: pattern.try_into_api()?,
            },
            Self::BannerDuplicate { banner, result } => mod_api::RecipeRule::BannerDuplicate {
                banner: banner.try_into_api()?,
                result: result.try_into_api()?,
            },
            Self::ShieldDecoration {
                banner,
                target,
                result,
            } => mod_api::RecipeRule::ShieldDecoration {
                banner: banner.try_into_api()?,
                target: target.try_into_api()?,
                result: result.try_into_api()?,
            },
            Self::Transmute {
                input,
                material,
                min,
                max,
                add_count,
                result,
            } => mod_api::RecipeRule::Transmute {
                input: input.try_into_api()?,
                material: material.try_into_api()?,
                min: *min,
                max: *max,
                add_count: *add_count,
                result: result.try_into_api()?,
            },
            Self::Dye {
                target,
                dye,
                result,
            } => mod_api::RecipeRule::Dye {
                target: target.try_into_api()?,
                dye: dye.try_into_api()?,
                result: result.try_into_api()?,
            },
            Self::Imbue {
                source,
                material,
                result,
            } => mod_api::RecipeRule::Imbue {
                source: source.try_into_api()?,
                material: material.try_into_api()?,
                result: result.try_into_api()?,
            },
            Self::BookCloning {
                source,
                material,
                min,
                max,
                result,
            } => mod_api::RecipeRule::BookCloning {
                source: source.try_into_api()?,
                material: material.try_into_api()?,
                min: *min,
                max: *max,
                result: result.try_into_api()?,
            },
            Self::FireworkRocket {
                shell,
                fuel,
                star,
                result,
            } => mod_api::RecipeRule::FireworkRocket {
                shell: shell.try_into_api()?,
                fuel: fuel.try_into_api()?,
                star: star.try_into_api()?,
                result: result.try_into_api()?,
            },
            Self::FireworkStar {
                dye,
                fuel,
                trail,
                twinkle,
                shapes,
                result,
            } => mod_api::RecipeRule::FireworkStar {
                dye: dye.try_into_api()?,
                fuel: fuel.try_into_api()?,
                trail: trail.try_into_api()?,
                twinkle: twinkle.try_into_api()?,
                shapes: shapes.try_into_api()?,
                result: result.try_into_api()?,
            },
            Self::FireworkStarFade {
                dye,
                target,
                result,
            } => mod_api::RecipeRule::FireworkStarFade {
                dye: dye.try_into_api()?,
                target: target.try_into_api()?,
                result: result.try_into_api()?,
            },
            Self::MapExtending {
                map,
                material,
                result,
            } => mod_api::RecipeRule::MapExtending {
                map: map.try_into_api()?,
                material: material.try_into_api()?,
                result: result.try_into_api()?,
            },
            Self::DecoratedPot {
                back,
                left,
                right,
                front,
                result,
            } => mod_api::RecipeRule::DecoratedPot {
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

impl TryIntoApi for crate::crafting::Ingredient {
    type Api = mod_api::Ingredient;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(mod_api::Ingredient {
            items: self.items.clone(),
            order: self.order.clone(),
            custom: self.custom.clone(),
            children: self
                .children
                .as_ref()
                .map(|(shape, kids)| Ok((shape.clone(), kids.try_into_api()?)))
                .transpose()?,
        })
    }
}
impl TryIntoEngine for mod_api::Ingredient {
    type Engine = crate::crafting::Ingredient;
    type Error = StackWriteError;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        Ok(crate::crafting::Ingredient {
            items: self.items.clone(),
            order: self.order.clone(),
            custom: self.custom.clone(),
            children: self
                .children
                .as_ref()
                .map(|(shape, kids)| {
                    Ok((
                        shape.clone(),
                        kids.iter()
                            .map(|k| k.try_into_engine(items))
                            .collect::<Result<_, _>>()?,
                    ))
                })
                .transpose()?,
        })
    }
}
impl TryIntoApi for crate::crafting::RecipeTemplate {
    type Api = mod_api::RecipeTemplate;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(mod_api::RecipeTemplate::new(
            self.item.clone(),
            self.count,
            self.components.clone(),
        ))
    }
}
impl TryIntoApi for crate::crafting::RecipeTrimPattern {
    type Api = mod_api::RecipeTrimPattern;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(match self {
            Self::Reference(id) => mod_api::RecipeTrimPattern::Reference(id.clone()),
            Self::Inline {
                asset_id,
                description,
                decal,
            } => mod_api::RecipeTrimPattern::Inline {
                asset_id: asset_id.clone(),
                description: description.clone(),
                decal: *decal,
            },
        })
    }
}
impl TryIntoApi for crate::crafting::FireworkShapeIngredient {
    type Api = mod_api::FireworkShapeIngredient;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(mod_api::FireworkShapeIngredient::new(
            self.shape,
            self.ingredient.try_into_api()?,
        ))
    }
}
impl TryIntoApi for crate::crafting::RecipeSnapshot {
    type Api = mod_api::Recipe;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        let mut result =
            mod_api::Recipe::new(self.id.clone(), self.kind.clone(), self.json.clone());
        result.group = self.group.clone();
        result.ingredients = self.ingredients.try_into_api()?;
        result.results = self.results.try_into_api()?;
        result.cooking_time = self.cooking_time;
        result.experience = self.experience;
        result.rule = self.rule.try_into_api()?;
        Ok(result)
    }
}

/// Install the engine-facing callbacks; snapshots never enter engine storage.
pub(crate) fn install_stack_hooks() {
    crate::native_stack_hooks::install(crate::native_stack_hooks::StackHooks {
        ingredient_test: |context, kind, data, stack, accept| {
            if !mod_api::__private::listened(mod_api::HookId::IngredientTest) {
                return;
            }
            if let Ok(stack) = stack.try_into_api() {
                mod_api::crafting::native_ingredient_test(context, kind, data, &stack, accept);
            }
        },
        enchantments: |stack, target, value| {
            if !mod_api::__private::listened(mod_api::HookId::GetEnchantmentLevel) {
                return;
            }
            if let Ok(stack) = stack.try_into_api() {
                mod_api::enchantments::native_get_enchantment_level(&stack, target, value);
            }
        },
        attributes: |stack, value| {
            if !mod_api::__private::listened(mod_api::HookId::ItemAttributeModifier) {
                return;
            }
            if let Ok(stack) = stack.try_into_api() {
                mod_api::item_attributes::native_item_attribute_modifier(&stack, value);
            }
        },
    });
}

struct ApiSlot<'a>(&'a mut dyn crate::menu::SlotView);
impl mod_api::menu::SlotView for ApiSlot<'_> {
    fn item(&self) -> Option<ApiStack> {
        self.0.item().and_then(|s| s.try_into_api().ok())
    }
    fn may_place(&self, stack: &ApiStack) -> bool {
        stack
            .try_into_engine(self.0.items())
            .is_ok_and(|s| self.0.may_place(&s))
    }
    fn may_pickup(&self) -> bool {
        self.0.may_pickup()
    }
    fn allow_modification(&self) -> bool {
        self.0.allow_modification()
    }
    fn safe_insert(&mut self, stack: Option<ApiStack>, count: u32) -> Option<ApiStack> {
        let value = match stack
            .as_ref()
            .map(|s| s.try_into_engine(self.0.items()))
            .transpose()
        {
            Ok(s) => s,
            Err(_) => return stack,
        };
        self.0
            .safe_insert(value, count)
            .and_then(|s| s.try_into_api().ok())
    }
    fn safe_take(&mut self, count: u32, max: u32) -> Option<ApiStack> {
        self.0
            .safe_take(count, max)
            .and_then(|s| s.try_into_api().ok())
    }
}
/// Adapt a synchronous slot event; engine slot rules still own every write.
pub(crate) fn stacked_on_other(
    context: &horizon_world::native_load_context::LoopContext,
    event: &mut crate::menu::StackedOnOther<'_>,
    canceled: &mut bool,
) {
    let Ok(mut carried) = event.carried.0.try_into_api() else {
        return;
    };
    let mut slot = ApiSlot(event.clicked);
    let mut api = mod_api::menu::StackedOnOther {
        player: event.player,
        player_entity_id: event.player_entity_id,
        slot: event.slot,
        container: event.container,
        container_slot: event.container_slot,
        primary: event.primary,
        carried: mod_api::menu::CarriedAccess(&mut carried),
        clicked: &mut slot,
        cancellation_result: event.cancellation_result,
    };
    mod_api::dispatch_item_stacked_on_other(&mod_api::OwningLoop::new(context), &mut api, canceled);
    event.cancellation_result = api.cancellation_result;
    if let Ok(value) = carried
        .as_ref()
        .map(|s| s.try_into_engine(slot.0.items()))
        .transpose()
    {
        *event.carried.0 = value;
    }
}

impl TryIntoApi for crate::stations::AnvilResult {
    type Api = mod_api::stations::AnvilResult;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(mod_api::stations::AnvilResult {
            result: self.result.try_into_api()?,
            cost: self.cost,
            repair_item_count_cost: self.repair_item_count_cost,
            only_renaming: self.only_renaming,
        })
    }
}
impl TryIntoEngine for mod_api::stations::AnvilResult {
    type Engine = crate::stations::AnvilResult;
    type Error = StackWriteError;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        Ok(crate::stations::AnvilResult {
            result: self.result.try_into_engine(items)?,
            cost: self.cost,
            repair_item_count_cost: self.repair_item_count_cost,
            only_renaming: self.only_renaming,
        })
    }
}
impl TryIntoApi for crate::menu::AnvilCraft {
    type Api = mod_api::menu::AnvilCraft;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(mod_api::menu::AnvilCraft {
            left: self.left.try_into_api()?,
            right: self.right.try_into_api()?,
            output: self.output.try_into_api()?,
        })
    }
}
impl TryIntoEngine for mod_api::menu::AnvilCraft {
    type Engine = crate::menu::AnvilCraft;
    type Error = StackWriteError;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        Ok(crate::menu::AnvilCraft {
            left: self.left.try_into_engine(items)?,
            right: self.right.try_into_engine(items)?,
            output: self.output.try_into_engine(items)?,
        })
    }
}
impl TryIntoApi for crate::menu::GrindstonePlace {
    type Api = mod_api::menu::GrindstonePlace;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(mod_api::menu::GrindstonePlace {
            top: self.top.try_into_api()?,
            bottom: self.bottom.try_into_api()?,
            vanilla_output: self.vanilla_output.try_into_api()?,
            output: self.output.try_into_api()?,
            xp: self.xp,
        })
    }
}
impl TryIntoEngine for mod_api::menu::GrindstonePlace {
    type Engine = crate::menu::GrindstonePlace;
    type Error = StackWriteError;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        Ok(crate::menu::GrindstonePlace {
            top: self.top.try_into_engine(items)?,
            bottom: self.bottom.try_into_engine(items)?,
            vanilla_output: self.vanilla_output.try_into_engine(items)?,
            output: self.output.try_into_engine(items)?,
            xp: self.xp,
        })
    }
}
impl TryIntoApi for crate::menu::GrindstoneTake {
    type Api = mod_api::menu::GrindstoneTake;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(mod_api::menu::GrindstoneTake {
            top: self.top.try_into_api()?,
            bottom: self.bottom.try_into_api()?,
            xp: self.xp,
            new_top: self.new_top.try_into_api()?,
            new_bottom: self.new_bottom.try_into_api()?,
        })
    }
}
impl TryIntoEngine for mod_api::menu::GrindstoneTake {
    type Engine = crate::menu::GrindstoneTake;
    type Error = StackWriteError;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        Ok(crate::menu::GrindstoneTake {
            top: self.top.try_into_engine(items)?,
            bottom: self.bottom.try_into_engine(items)?,
            xp: self.xp,
            new_top: self.new_top.try_into_engine(items)?,
            new_bottom: self.new_bottom.try_into_engine(items)?,
        })
    }
}

impl TryIntoApi for crate::menu::AnvilUpdate {
    type Api = mod_api::menu::AnvilUpdate;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        let vanilla = mod_api::stations::AnvilResult {
            result: self.vanilla.output.try_into_api()?,
            cost: self.vanilla.xp_cost,
            repair_item_count_cost: self.vanilla.material_cost.max(0) as u32,
            only_renaming: false,
        };
        let mut result = mod_api::menu::AnvilUpdate::new(
            self.left.try_into_api()?,
            self.right.try_into_api()?,
            self.name.clone(),
            &vanilla,
        );
        result.output = self.output.try_into_api()?;
        result.xp_cost = self.xp_cost;
        result.material_cost = self.material_cost;
        Ok(result)
    }
}

impl TryIntoEngine for mod_api::menu::AnvilUpdate {
    type Engine = crate::menu::AnvilUpdate;
    type Error = StackWriteError;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        Ok(crate::menu::AnvilUpdate {
            left: self.left.try_into_engine(items)?,
            right: self.right.try_into_engine(items)?,
            name: self.name.clone(),
            vanilla: crate::menu::AnvilVanilla {
                output: self.vanilla.output.try_into_engine(items)?,
                xp_cost: self.vanilla.xp_cost,
                material_cost: self.vanilla.material_cost,
            },
            output: self.output.try_into_engine(items)?,
            xp_cost: self.xp_cost,
            material_cost: self.material_cost,
        })
    }
}
impl super::Game<'_> {
    pub(in crate::game) fn api_anvil_update(
        &mut self,
        player: u128,
        entity: i32,
        pos: [i32; 3],
        value: &mut crate::menu::AnvilUpdate,
        cancel: &mut bool,
    ) {
        let Ok(mut api) = value.try_into_api() else {
            return;
        };
        self.native_anvil_update(player, entity, pos, &mut api, cancel);
        match api.try_into_engine(self.items()) {
            Ok(result) => *value = result,
            Err(_) => *cancel = true,
        }
    }
    pub(in crate::game) fn api_grindstone_place(
        &mut self,
        player: u128,
        entity: i32,
        pos: [i32; 3],
        value: &mut crate::menu::GrindstonePlace,
        cancel: &mut bool,
    ) {
        let Ok(mut api) = value.try_into_api() else {
            return;
        };
        self.native_grindstone_place(player, entity, pos, &mut api, cancel);
        match api.try_into_engine(self.items()) {
            Ok(result) => *value = result,
            Err(_) => *cancel = true,
        }
    }
    pub(in crate::game) fn api_grindstone_take(
        &mut self,
        player: u128,
        entity: i32,
        pos: [i32; 3],
        value: &mut crate::menu::GrindstoneTake,
        cancel: &mut bool,
    ) {
        let Ok(mut api) = value.try_into_api() else {
            return;
        };
        self.native_grindstone_take(player, entity, pos, &mut api, cancel);
        match api.try_into_engine(self.items()) {
            Ok(result) => *value = result,
            Err(_) => *cancel = true,
        }
    }
    pub(in crate::game) fn api_anvil_craft_pre(
        &mut self,
        player: u128,
        name: &str,
        pos: [i32; 3],
        value: &crate::menu::AnvilCraft,
        cancel: &mut bool,
    ) {
        if let Ok(api) = value.try_into_api() {
            self.native_anvil_craft_pre(player, name, pos, &api, cancel);
        }
    }
    pub(in crate::game) fn api_anvil_craft_post(
        &mut self,
        player: u128,
        name: &str,
        pos: [i32; 3],
        value: &crate::menu::AnvilCraft,
    ) {
        if let Ok(api) = value.try_into_api() {
            self.native_anvil_craft_post(player, name, pos, &api);
        }
    }
}

#[cfg(test)]
mod attribute_boundary_tests {
    use super::EngineStack as ItemStack;
    use crate::item_attributes::attribute_modifiers_or;
    use horizon_items::item_data::ModifierRow;
    use mod_api::{
        AttributeModifier, AttributeOperation, EquipmentSlotGroup, Hooks, Identifier,
        ItemAttributeModifierEvent, ModEntry, testing::with_mods,
    };
    use std::borrow::Cow;
    /// Makes a netherite chestplate heavier: +0.4 knockback resistance in
    /// the chest slot, and no armor toughness.
    struct Heavy;
    #[mod_api::__subscriptions]
    impl Hooks for Heavy {
        fn item_attribute_modifier(&self, event: &mut ItemAttributeModifierEvent<'_>) {
            if mod_api::__private::item_key_id(event.stack.item())
                != Some("minecraft:netherite_chestplate")
            {
                return;
            }
            let id = |text: &str| Identifier::parse(text).unwrap();
            let heavy = |amount| {
                AttributeModifier::new(id("test:heavy"), amount, AttributeOperation::AddValue)
            };
            let resistance = id("minecraft:knockback_resistance");
            assert!(
                event
                    .modifiers
                    .add(resistance.clone(), heavy(0.4), EquipmentSlotGroup::Chest)
            );
            // The id is taken on that attribute.
            assert!(
                !event
                    .modifiers
                    .add(resistance, heavy(1.0), EquipmentSlotGroup::Chest)
            );
            assert!(
                event
                    .modifiers
                    .remove_all_for(&id("minecraft:armor_toughness"))
            );
        }
    }

    fn chestplate() -> Vec<ModifierRow> {
        ["minecraft:armor", "minecraft:armor_toughness"]
            .map(|attribute| {
                ModifierRow::new(
                    attribute,
                    "minecraft:armor.chestplate",
                    3.0,
                    "add_value",
                    "chest",
                )
            })
            .to_vec()
    }

    /// A mod's edits are what the stack grants; a stack it leaves alone, and
    /// any stack with no mod listening, keeps the item's list borrowed.
    #[test]
    fn a_mod_edits_what_a_stack_grants() {
        super::prepare_stack_boundary_tests();
        let types = chestplate();
        let netherite = ItemStack::new("minecraft:netherite_chestplate", 1);
        let diamond = ItemStack::new("minecraft:diamond_chestplate", 1);
        with_mods(Vec::new().into(), || {
            assert!(matches!(
                attribute_modifiers_or(&netherite, &types),
                Cow::Borrowed(_)
            ));
        });
        with_mods(vec![ModEntry::new("heavy", Box::new(Heavy))].into(), || {
            assert_eq!(
                &*attribute_modifiers_or(&netherite, &types),
                [
                    types[0].clone(),
                    ModifierRow::new(
                        "minecraft:knockback_resistance",
                        "test:heavy",
                        0.4,
                        "add_value",
                        "chest"
                    ),
                ]
            );
            assert!(matches!(
                attribute_modifiers_or(&diamond, &types),
                Cow::Borrowed(_)
            ));
        });
    }
}

#[cfg(test)]
mod enchantment_boundary_tests {
    use super::EngineStack as ItemStack;
    use crate::enchantment_levels::get_enchantment_level;
    use mod_api::{GetEnchantmentLevelEvent, Hooks, Identifier, ModEntry, testing::with_mods};
    use std::borrow::Cow;
    use std::sync::Arc;
    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }
    /// An iron sword fights as if it had Looting II and no Sharpness.
    struct Keen;
    #[mod_api::__subscriptions]
    impl Hooks for Keen {
        fn get_enchantment_level(&self, event: &mut GetEnchantmentLevelEvent<'_>) {
            if mod_api::__private::item_key_id(event.stack.item()) != Some("minecraft:iron_sword") {
                return;
            }
            let looting = id("minecraft:looting");
            if event.target.is_none_or(|target| *target == looting) {
                event.enchantments.upgrade(looting, 2);
            }
            event.enchantments.set(id("minecraft:sharpness"), 0);
        }
    }

    /// Gameplay reads see the mod's levels; the stack's own component, a
    /// stack the mod leaves alone and every stack with no mod listening
    /// keep theirs, borrowed.
    #[test]
    fn a_mod_changes_the_levels_gameplay_reads() {
        super::prepare_stack_boundary_tests();
        let _ = horizon_items::native_enchantment_hooks::install(get_enchantment_level);
        let mut iron = ItemStack::new("minecraft:iron_sword", 1);
        iron.enchant("minecraft:sharpness", 3);
        let mut diamond = ItemStack::new("minecraft:diamond_sword", 1);
        diamond.enchant("minecraft:sharpness", 3);
        with_mods(Vec::new().into(), || {
            assert!(matches!(iron.all_enchantments(), Cow::Borrowed(_)));
            assert_eq!(iron.enchantment_level("minecraft:sharpness"), 3);
        });
        with_mods(vec![ModEntry::new("keen", Box::new(Keen))].into(), || {
            assert_eq!(
                &*iron.all_enchantments(),
                [(Arc::from("minecraft:looting"), 2)]
            );
            assert_eq!(iron.enchantment_level("minecraft:looting"), 2);
            assert_eq!(iron.enchantment_level("minecraft:sharpness"), 0);
            assert_eq!(iron.tag_enchantment_level("minecraft:sharpness"), 3);
            assert_eq!(iron.enchantments(), [(Arc::from("minecraft:sharpness"), 3)]);
            assert!(matches!(diamond.all_enchantments(), Cow::Borrowed(_)));
            assert_eq!(diamond.enchantment_level("minecraft:sharpness"), 3);
        });
    }
}

#[cfg(test)]
fn prepare_stack_boundary_tests() {
    install_stack_hooks();
    mod_api::__private::install_parts(&[]).unwrap();
    mod_api::__private::resolve_items([
        "minecraft:iron_sword",
        "minecraft:diamond_sword",
        "minecraft:netherite_chestplate",
        "minecraft:diamond_chestplate",
        "minecraft:potion",
    ])
    .unwrap();
}

#[cfg(test)]
mod brewed_boundary_tests {
    use super::*;
    struct NoHost;
    impl mod_api::Host for NoHost {}
    mod_api::impl_host_areas!(NoHost);
    impl mod_api::ServerHost for NoHost {}
    mod_api::impl_server_host_areas!(NoHost);
    struct Watch(std::sync::Arc<std::sync::Mutex<Vec<(u128, i32, String, u32)>>>);
    #[mod_api::__subscriptions]
    impl mod_api::Hooks for Watch {
        fn player_brewed_potion(
            &self,
            _: &mut dyn mod_api::ServerHost,
            event: &mut mod_api::PlayerBrewedPotionEvent<'_>,
        ) {
            self.0.lock().unwrap().push((
                event.player,
                event.player_entity_id,
                mod_api::__private::item_key_id(event.stack.item())
                    .unwrap()
                    .to_owned(),
                event.stack.count(),
            ));
        }
    }
    #[test]
    fn brewed_event_reports_player_and_converted_stack() {
        prepare_stack_boundary_tests();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        mod_api::testing::with_mods(
            vec![mod_api::ModEntry::new(
                "watch",
                Box::new(Watch(seen.clone())),
            )]
            .into(),
            || {
                let stack = EngineStack::new("minecraft:potion", 1)
                    .try_into_api()
                    .unwrap();
                mod_api::dispatch_player_brewed_potion(&mut NoHost, 7, 41, &stack);
            },
        );
        assert_eq!(
            *seen.lock().unwrap(),
            [(7, 41, "minecraft:potion".to_owned(), 1)]
        );
    }
}

impl TryIntoApi for crate::containers::ModBlockEntityFields {
    type Api = mod_api::containers::ModBlockEntityFields;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(mod_api::containers::ModBlockEntityFields {
            nbt: self.nbt.clone(),
            items: self.items.try_into_api()?,
            data: self.data.clone(),
        })
    }
}
impl TryIntoEngine for mod_api::containers::ModBlockEntityFields {
    type Engine = crate::containers::ModBlockEntityFields;
    type Error = StackWriteError;
    fn try_into_engine(&self, items: &Items) -> Result<Self::Engine, Self::Error> {
        Ok(crate::containers::ModBlockEntityFields {
            nbt: self.nbt.clone(),
            items: self.items.try_into_engine(items)?,
            data: self.data.clone(),
        })
    }
}
impl TryIntoApi for crate::containers::ModBlockEntity {
    type Api = mod_api::ModBlockEntity;
    type Error = StackReadError;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(mod_api::ModBlockEntity::from_snapshot(
            self.id().to_owned(),
            (**self).try_into_api()?,
            mod_api::containers::ModRevision::from_snapshot(
                self.revision().identity(),
                self.revision().changes(),
            ),
        ))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum BlockEntitySnapshotError {
    InvalidRevision,
    InvalidStack,
}

fn apply_block_entity_snapshot(
    entity: &mut crate::containers::ModBlockEntity,
    snapshot: &mod_api::ModBlockEntity,
    before: mod_api::containers::ModRevision,
    items: &Items,
) -> Result<(), BlockEntitySnapshotError> {
    let current = entity.revision();
    let revision = snapshot.revision();
    if snapshot.id() != entity.id()
        || revision.identity() != current.identity()
        || revision.changes() < current.changes()
    {
        return Err(BlockEntitySnapshotError::InvalidRevision);
    }
    if revision != before {
        let fields = (**snapshot)
            .try_into_engine(items)
            .map_err(|_| BlockEntitySnapshotError::InvalidStack)?;
        entity.apply_snapshot(fields, snapshot.revision().changes());
    }
    Ok(())
}

impl super::Game<'_> {
    pub(in crate::game) fn api_block_entity_tick(
        &mut self,
        pos: [i32; 3],
        entity: &mut crate::containers::ModBlockEntity,
    ) {
        let Ok(mut api) = entity.try_into_api() else {
            return;
        };
        let revision = api.revision();
        self.native_block_entity_tick(pos, &mut api);
        let _ = apply_block_entity_snapshot(entity, &api, revision, self.items());
    }
    pub(in crate::game) fn api_block_entity_update_tag(
        pos: [i32; 3],
        entity: &crate::containers::ModBlockEntity,
        tag: &mut Option<horizon_core::nbt::Compound>,
    ) {
        if let Ok(api) = entity.try_into_api() {
            Self::native_block_entity_update_tag(pos, &api, tag);
        }
    }
    pub(in crate::game) fn api_block_entity_submit(
        &self,
        pos: [i32; 3],
        block: &str,
        state: horizon_world::registry::BlockStateId,
        entity: &crate::containers::ModBlockEntity,
        draw: &mut super::native_block_entity_draw::BlockEntityDraw,
    ) {
        if let Ok(api) = entity.try_into_api() {
            self.native_block_entity_submit(pos, block, state, &api, draw);
        }
    }
    pub(in crate::game) fn api_block_entity_render_bounding_box(
        &self,
        pos: [i32; 3],
        block: &str,
        state: horizon_world::registry::BlockStateId,
        entity: &crate::containers::ModBlockEntity,
        bounds: &mut [f32; 6],
    ) {
        if let Ok(api) = entity.try_into_api() {
            self.native_block_entity_render_bounding_box(pos, block, state, &api, bounds);
        }
    }
    pub(in crate::game) fn api_menu_button(
        &mut self,
        context: &mod_api::menu::MenuButtonContext,
        entity: &mut crate::containers::ModBlockEntity,
        carried: &mut Option<EngineStack>,
    ) {
        let (Ok(mut api), Ok(mut cursor)) = (entity.try_into_api(), carried.try_into_api()) else {
            return;
        };
        let revision = api.revision();
        self.native_menu_button(context, &mut api, &mut cursor);
        if let Ok(cursor) = cursor.try_into_engine(self.items())
            && apply_block_entity_snapshot(entity, &api, revision, self.items()).is_ok()
        {
            *carried = cursor;
        }
    }
    pub(in crate::game) fn api_menu_quick_move(
        &mut self,
        pos: [i32; 3],
        entity: &mut crate::containers::ModBlockEntity,
        container_slots: usize,
        slot: usize,
        stack: &mut EngineStack,
        handled: &mut bool,
    ) {
        let (Ok(mut api), Ok(mut held)) = (entity.try_into_api(), stack.try_into_api()) else {
            return;
        };
        let revision = api.revision();
        self.native_menu_quick_move(pos, &mut api, container_slots, slot, &mut held, handled);
        if let Ok(held) = held.try_into_engine(self.items())
            && apply_block_entity_snapshot(entity, &api, revision, self.items()).is_ok()
        {
            *stack = held;
        } else {
            *handled = false;
        }
    }
}
impl super::GameLevel<'_> {
    pub(in crate::game) fn api_pre_remove_side_effects(
        &mut self,
        pos: horizon_core::position::BlockPos,
        state: horizon_world::registry::BlockStateId,
        entity: &crate::containers::ModBlockEntity,
        drops: &mut Vec<EngineStack>,
    ) {
        let Ok(api) = entity.try_into_api() else {
            return;
        };
        let mut added = Vec::new();
        self.native_pre_remove_side_effects(pos, state, &api, &mut added);
        if let Ok(added) = added.try_into_engine(self.items) {
            drops.extend(added);
        }
    }
}

#[cfg(test)]
mod block_entity_snapshot_tests {
    use super::*;
    #[test]
    fn unchanged_snapshots_keep_revision_and_callback_writes_advance_it() {
        let mut entity =
            crate::containers::ModBlockEntity::new("test:machine".into(), Default::default());
        let first = entity.try_into_api().unwrap();
        let mut second = entity.try_into_api().unwrap();
        assert_eq!(first.revision(), second.revision());
        let before = second.revision();
        second.data.push(7);
        second.data.push(8);
        assert_ne!(before, second.revision());
        apply_block_entity_snapshot(
            &mut entity,
            &second,
            before,
            &Items::for_tests(Default::default()),
        )
        .unwrap();
        let third = entity.try_into_api().unwrap();
        assert_ne!(first.revision(), third.revision());
        assert_eq!(second.revision(), third.revision());
        assert_eq!(third.revision(), entity.try_into_api().unwrap().revision());
        assert_eq!(entity.id(), "test:machine");
        assert_eq!(entity.data, [7, 8]);
        let other = mod_api::ModBlockEntity::new("test:machine".into(), Default::default());
        assert_ne!(other.revision(), third.revision());
        entity.data.push(9);
        let fourth = entity.try_into_api().unwrap();
        assert_ne!(fourth.revision(), second.revision());
        let items = Items::for_tests(Default::default());
        assert_eq!(
            apply_block_entity_snapshot(&mut entity, &second, before, &items),
            Err(BlockEntitySnapshotError::InvalidRevision)
        );
        assert_eq!(
            apply_block_entity_snapshot(&mut entity, &other, before, &items),
            Err(BlockEntitySnapshotError::InvalidRevision)
        );
        assert_eq!(entity.data, [7, 8, 9]);
    }
}

impl TryIntoApi for horizon_mod_package::Side {
    type Api = mod_api::Side;
    type Error = std::convert::Infallible;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(match self {
            Self::Client => mod_api::Side::Client,
            Self::Both => mod_api::Side::Both,
        })
    }
}

fn package_snapshot(
    manifest: &horizon_mod_package::Manifest,
    kind: mod_api::PackageKind,
) -> mod_api::PackageInfo {
    mod_api::PackageInfo::new(
        manifest.id.clone(),
        manifest.version.clone(),
        kind,
        manifest.side.try_into_api().unwrap(),
        std::iter::once(&manifest.id)
            .chain(&manifest.namespaces)
            .cloned()
            .collect(),
        manifest.dependencies.keys().cloned().collect(),
    )
}
impl TryIntoApi for horizon_mod_package::Manifest {
    type Api = mod_api::PackageInfo;
    type Error = std::convert::Infallible;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(package_snapshot(
            self,
            if self.wasm.is_some() {
                mod_api::PackageKind::Code
            } else {
                mod_api::PackageKind::Content
            },
        ))
    }
}
impl TryIntoApi for horizon_mod_package::packages::AdjacentManifest {
    type Api = mod_api::PackageInfo;
    type Error = std::convert::Infallible;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        Ok(package_snapshot(&self.manifest, mod_api::PackageKind::Code))
    }
}

pub(in crate::game) fn install_packages(
    native: &[horizon_mod_package::packages::AdjacentManifest],
    content: &[horizon_mod_package::selection::SelectedPackage],
) {
    let native: Vec<_> = native.iter().map(|p| p.try_into_api().unwrap()).collect();
    let content: Vec<_> = content
        .iter()
        .map(|p| p.package.manifest.try_into_api().unwrap())
        .collect();
    mod_api::__private::install_packages(&native, &content);
}

#[cfg(test)]
mod package_snapshot_tests {
    use super::*;
    #[test]
    fn package_metadata_keeps_declared_side_and_namespace_order() {
        for (side, expected) in [
            ("client", mod_api::Side::Client),
            ("both", mod_api::Side::Both),
        ] {
            let manifest = horizon_mod_package::Manifest::parse(&format!(
                "id='test'\nname='Test'\nversion='1'\napi='0.2'\nside='{side}'\ncapabilities=['registries']\n"
            )).unwrap();
            let snapshot = manifest.try_into_api().unwrap();
            assert_eq!(snapshot.side(), expected);
            assert_eq!(snapshot.namespaces().collect::<Vec<_>>(), ["test"]);
            assert_eq!(snapshot.kind(), mod_api::PackageKind::Content);
        }
    }
}

impl TryIntoApi for horizon_client_core::suggestion::Suggestion {
    type Api = mod_api::Suggestion;
    type Error = std::convert::Infallible;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error> {
        let mut value = mod_api::Suggestion::new(self.range, self.text.clone());
        value.tooltip.clone_from(&self.tooltip);
        value.int_value = self.int_value;
        Ok(value)
    }
}
impl TryIntoEngine for mod_api::Suggestion {
    type Engine = horizon_client_core::suggestion::Suggestion;
    type Error = StackWriteError;
    fn try_into_engine(&self, _: &Items) -> Result<Self::Engine, Self::Error> {
        Ok(Self::Engine {
            range: self.range,
            text: self.text.clone(),
            tooltip: self.tooltip.clone(),
            int_value: self.int_value,
        })
    }
}
#[cfg(test)]
mod completion_snapshot_tests {
    use super::*;
    #[test]
    fn completion_round_trip_keeps_range_tooltip_and_numeric_order() {
        let source = horizon_client_core::suggestion::Suggestion {
            range: (4, 7),
            text: "42".into(),
            tooltip: Some("count".into()),
            int_value: Some(42),
        };
        let mut api = source.try_into_api().unwrap();
        api.text = "43".into();
        api.int_value = Some(43);
        let engine = api
            .try_into_engine(&Items::for_tests(Default::default()))
            .unwrap();
        assert_eq!(engine.range, (4, 7));
        assert_eq!(engine.tooltip.as_deref(), Some("count"));
        assert_eq!(engine.int_value, Some(43));
        assert_eq!(engine.apply("foo 123 bar"), "foo 43 bar");
        assert_eq!(source.text, "42");
    }
}

// Registration declarations do not need a live item registry.
impl TryIntoEngine<()> for mod_api::BlockEntityTypeDecl {
    type Engine = horizon_mod_package::content::BlockEntityType;
    type Error = std::convert::Infallible;
    fn try_into_engine(&self, _: &()) -> Result<Self::Engine, Self::Error> {
        Ok(Self::Engine {
            id: self.id.clone(),
            blocks: self.blocks.clone(),
        })
    }
}
pub(in crate::game) fn block_entity_declarations()
-> Vec<horizon_mod_package::content::BlockEntityType> {
    mod_api::__private::block_entity_types()
        .iter()
        .map(|decl| decl.try_into_engine(&()).unwrap())
        .collect()
}

#[cfg(test)]
mod declaration_snapshot_tests {
    use super::*;
    #[test]
    fn block_entity_declarations_keep_supported_block_order() {
        // Use registration, the owner of declaration validation.
        struct Mod;
        #[mod_api::__subscriptions]
        impl mod_api::Hooks for Mod {
            fn register(&self, r: &mut dyn mod_api::Registrar, _: &mod_api::RegistrationContext) {
                r.declare_block_entity_type(
                    "machine",
                    vec!["test:second".into(), "test:first".into()],
                );
            }
        }
        mod_api::__private::install_parts(&[mod_api::ModEntry::new("test", Box::new(Mod))])
            .unwrap();
        let declarations = block_entity_declarations();
        assert_eq!(declarations[0].id, "test:machine");
        assert_eq!(declarations[0].blocks, ["test:second", "test:first"]);
    }
}
