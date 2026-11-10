//! Shared augmenting-path recipe assignment.
use super::*;

/// `StackedContents.RecipePicker`: augmenting paths over item-ingredient pairs.
pub(super) struct Picker<'a> {
    stacked: &'a mut Stacked,
    ingredients: Vec<&'a Ingredient>,
    /// Indices into the stacked keys of the items any ingredient accepts.
    items: Vec<usize>,
    connection: Vec<bool>,
    assigned: Vec<bool>,
    satisfied: Vec<bool>,
    visited_ingredient: Vec<bool>,
    visited_item: Vec<bool>,
    path: Vec<usize>,
}

impl<'a> Picker<'a> {
    pub(super) fn new(stacked: &'a mut Stacked, ingredients: &[&'a Ingredient]) -> Self {
        // getUniqueAvailableIngredientItems
        let items: Vec<usize> = (0..stacked.keys.len())
            .filter(|&i| {
                stacked.amounts[i] > 0
                    && ingredients
                        .iter()
                        .any(|g| g.items.contains(&*stacked.keys[i]))
            })
            .collect();
        let (ni, nt) = (ingredients.len(), items.len());
        let mut connection = vec![false; ni * nt];
        for (g, ing) in ingredients.iter().enumerate() {
            for (t, &key) in items.iter().enumerate() {
                if ing.items.contains(&*stacked.keys[key]) {
                    connection[t * ni + g] = true;
                }
            }
        }
        Self {
            stacked,
            ingredients: ingredients.to_vec(),
            items,
            connection,
            assigned: vec![false; ni * nt],
            satisfied: vec![false; ni],
            visited_ingredient: vec![false; ni],
            visited_item: vec![false; nt],
            path: Vec::new(),
        }
    }
    fn at(&self, item: usize, ingredient: usize) -> usize {
        item * self.ingredients.len() + ingredient
    }
    fn clear_visited(&mut self) {
        self.visited_ingredient.fill(false);
        self.visited_item.fill(false);
    }
    pub(super) fn try_pick(&mut self, capacity: i32, mut output: Option<&mut Vec<Arc<str>>>) -> bool {
        if capacity <= 0 {
            return true;
        }
        let mut satisfied_count = 0;
        loop {
            let Some(path) = self.try_assigning_new_item(capacity) else {
                let valid = satisfied_count == self.ingredients.len();
                let has_output = valid && output.is_some();
                self.clear_visited();
                self.satisfied.fill(false);
                for g in 0..self.ingredients.len() {
                    for t in 0..self.items.len() {
                        let at = self.at(t, g);
                        if self.assigned[at] {
                            self.assigned[at] = false;
                            let key = self.items[t];
                            self.stacked.amounts[key] += capacity;
                            if has_output && let Some(out) = output.as_deref_mut() {
                                out.push(self.stacked.keys[key].clone());
                            }
                            break;
                        }
                    }
                }
                return valid;
            };
            let key = self.items[path[0]];
            self.stacked.amounts[key] -= capacity;
            self.satisfied[path[path.len() - 1]] = true;
            satisfied_count += 1;
            for i in 0..path.len() - 1 {
                if i & 1 == 0 {
                    let at = self.at(path[i], path[i + 1]);
                    self.assigned[at] = true;
                } else {
                    let at = self.at(path[i + 1], path[i]);
                    self.assigned[at] = false;
                }
            }
        }
    }
    fn try_assigning_new_item(&mut self, capacity: i32) -> Option<Vec<usize>> {
        self.clear_visited();
        for t in 0..self.items.len() {
            if self.stacked.amounts[self.items[t]] >= capacity
                && let Some(path) = self.find_path(t)
            {
                return Some(path);
            }
        }
        None
    }
    fn find_path(&mut self, start: usize) -> Option<Vec<usize>> {
        self.path.clear();
        self.visited_item[start] = true;
        self.path.push(start);
        while !self.path.is_empty() {
            let len = self.path.len();
            if (len - 1) & 1 == 0 {
                let item = self.path[len - 1];
                for g in 0..self.ingredients.len() {
                    let at = self.at(item, g);
                    if !self.visited_ingredient[g] && self.connection[at] && !self.assigned[at] {
                        self.visited_ingredient[g] = true;
                        self.path.push(g);
                        break;
                    }
                }
            } else {
                let ingredient = self.path[len - 1];
                if !self.satisfied[ingredient] {
                    return Some(self.path.clone());
                }
                for t in 0..self.items.len() {
                    if !self.visited_item[t] && self.assigned[self.at(t, ingredient)] {
                        self.visited_item[t] = true;
                        self.path.push(t);
                        break;
                    }
                }
            }
            if self.path.len() == len {
                self.path.pop();
            }
        }
        None
    }
    pub(super) fn try_pick_all(&mut self, max_size: i32, output: Option<&mut Vec<Arc<str>>>) -> i32 {
        let mut min = 0i32;
        // Java int arithmetic: an empty ingredient list overflows as vanilla does.
        let mut max = max_size
            .min(self.stacked.upper_bound(&self.ingredients))
            .wrapping_add(1);
        loop {
            let mid = min.wrapping_add(max) / 2;
            if self.try_pick(mid, None) {
                if max.wrapping_sub(min) <= 1 {
                    if mid > 0 {
                        self.try_pick(mid, output);
                    }
                    return mid;
                }
                min = mid;
            } else {
                max = mid;
            }
        }
    }
}

