use itertools::Itertools;

use crate::{
    game_data::{Crafter, Recipe},
    rf,
};

pub struct MachinePrototype {}

#[derive(Debug, Clone)]
pub struct Machine {
    // TODO: non-crafting machines?
    pub crafter: Crafter,
    pub crafter_count: f64,
    pub recipe: Recipe,
}

#[derive(Debug, Clone)]
pub struct ItemSpeed {
    pub item: String,
    pub speed: f64,
}

impl Machine {
    pub fn crafts_per_second(&self) -> f64 {
        self.crafter.crafting_speed * self.crafter_count / self.recipe.energy
    }

    pub fn item_speeds(&self) -> Vec<ItemSpeed> {
        let crafts_per_second = self.crafts_per_second();
        self.recipe
            .ingredients
            .iter()
            .map(|ing| ItemSpeed {
                item: ing.name.clone(),
                speed: -crafts_per_second * ing.amount,
            })
            .chain(self.recipe.products.iter().map(|product| ItemSpeed {
                item: product.name.clone(),
                speed: crafts_per_second * product.amount,
            }))
            .collect()
    }

    pub fn print_io(&self) {
        let crafts_per_second = self.crafts_per_second();

        let inputs = self
            .recipe
            .ingredients
            .iter()
            .map(|ing| {
                let items_per_sec = crafts_per_second * ing.amount;
                format!("{}/s {}", rf(items_per_sec), ing.name)
            })
            .join(" + ");

        let outputs = self
            .recipe
            .products
            .iter()
            .map(|ing| {
                let items_per_sec = crafts_per_second * ing.amount;
                format!("{}/s {}", rf(items_per_sec), ing.name)
            })
            .join(" + ");

        println!(
            "{} -> [{} × {}] -> {}",
            inputs,
            rf(self.crafter_count),
            self.crafter.name,
            outputs
        );
    }
}
