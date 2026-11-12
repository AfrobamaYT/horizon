use super::*;

/// `RepairItemRecipe.canCombine`.
pub(super) fn pair<'a>(
    ctx: &Context,
    stacks: &[&'a ItemStack],
) -> Option<(&'a ItemStack, &'a ItemStack)> {
    let [first, second] = stacks else {
        return None;
    };
    let durable = |s: &ItemStack| {
        let info = ctx.facts(s).map(|f| &f.info);
        s.carries_component(info, "minecraft:max_damage")
            && s.carries_component(info, "minecraft:damage")
    };
    (first.item == second.item
        && first.count == 1
        && second.count == 1
        && durable(first)
        && durable(second))
    .then_some((*first, *second))
}

/// `RepairItemRecipe.assemble`: integer division truncates the five percent bonus.
pub(super) fn assemble(ctx: &Context, stacks: &[&ItemStack]) -> Option<ItemStack> {
    let (first, second) = pair(ctx, stacks)?;
    let info = |s: &ItemStack| ctx.facts(s).map(|f| &f.info);
    let max = |s: &ItemStack| i64::from(s.max_damage(info(s)));
    let durability = max(first).max(max(second));
    let remaining = (max(first) - i64::from(first.effective_damage(info(first))))
        + (max(second) - i64::from(second.effective_damage(info(second))))
        + durability * 5 / 100;
    let mut out = ItemStack::new(&first.item, 1);
    out.data_mut()
        .set_other("minecraft:max_damage", Some(Tag::Int(durability as i32)));
    out.damage = (durability - remaining).max(0) as u32;
    // Only the curses of both survive, at the higher level.
    let (a, b) = (
        first.crafting_enchantments(),
        second.crafting_enchantments(),
    );
    let mut curses: Vec<(Arc<str>, u32)> = Vec::new();
    for (id, level) in a.iter().chain(b.iter()) {
        if !ctx.recipes.curses.contains(&**id) {
            continue;
        }
        match curses.iter_mut().find(|(c, _)| c == id) {
            Some((_, l)) => *l = (*l).max(*level),
            None => curses.push((id.clone(), *level)),
        }
    }
    out.set_crafting_enchantments(curses);
    Some(out)
}

#[cfg(test)]
#[path = "repair_item_tests.rs"]
mod tests;
