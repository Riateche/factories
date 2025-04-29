use itertools::Itertools;

use crate::game_data::{Crafter, Recipe};

pub struct MachinePrototype {}

#[derive(Debug, Clone)]
pub struct Machine {
    // TODO: non-crafting machines?
    pub crafter: Crafter,
    pub crafter_count: f64,
    pub recipe: Recipe,
}

impl Machine {
    pub fn print_io(&self) {
        let crafts_per_second =
            self.crafter.crafting_speed * self.crafter_count / self.recipe.energy;

        let inputs = self
            .recipe
            .ingredients
            .iter()
            .map(|ing| {
                let items_per_sec = crafts_per_second * ing.amount;
                format!("{:.2}/s {}", items_per_sec, ing.name)
            })
            .join(" + ");

        let outputs = self
            .recipe
            .products
            .iter()
            .map(|ing| {
                let items_per_sec = crafts_per_second * ing.amount;
                format!("{:.2}/s {}", items_per_sec, ing.name)
            })
            .join(" + ");

        println!("{inputs} -> {outputs}");
    }
}
