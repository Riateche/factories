#![allow(dead_code)]

use {crate::info::Info, itertools::Itertools, std::collections::BTreeSet, tracing::trace};

// Adjust as necessary.
const REACHABLE_RESOURCES: Option<&[&str]> = Some(&[
    "coal",
    "copper-ore",
    "crude-oil",
    "iron-ore",
    "stone",
    "water",
    "wood",
]);

const HARVESTABLE_LIQUIDS: &[&str] = &["water", "lava", "heavy-oil", "ammoniacal-solution"];

pub fn reachable_items(info: &Info) -> BTreeSet<String> {
    let harvestable_resources: BTreeSet<String> = info
        .game_data
        .entities
        .values()
        .filter(|v| v.resource_category.is_some() || v.type_ == "plant" || v.type_ == "tree")
        .flat_map(|v| &v.mineable_properties.as_ref().unwrap().products)
        .map(|v| v.name.clone())
        .chain(HARVESTABLE_LIQUIDS.iter().map(|v| v.to_string()))
        .collect();
    trace!("harvestable_resources: {harvestable_resources:?}\n");

    let mut reachable_items: BTreeSet<String> = if let Some(resources) = REACHABLE_RESOURCES {
        resources.iter().map(|s| s.to_string()).collect()
    } else {
        harvestable_resources
    };

    let mut verified_recipes = BTreeSet::new();
    let mut i = 0;
    loop {
        i += 1;
        let mut new_reachable_items = BTreeSet::new();

        for recipe in info.game_data.recipes.values() {
            if recipe
                .ingredients
                .iter()
                .all(|ing| reachable_items.contains(&ing.name))
            {
                for product in &recipe.products {
                    if !reachable_items.contains(&product.name) {
                        trace!(
                            "{} | {} -> {}",
                            recipe.name,
                            recipe.ingredients.iter().map(|ing| &ing.name).join(", "),
                            product.name
                        );
                        new_reachable_items.insert(product.name.clone());
                    }
                }
                if !verified_recipes.contains(&recipe.name) {
                    if let Some(bad_product) = recipe
                        .products
                        .iter()
                        .find(|product| reachable_items.contains(&product.name))
                    {
                        trace!("loop detected for {}: {recipe:?}\n\n", bad_product.name);
                    }
                    verified_recipes.insert(recipe.name.to_string());
                }
            }
        }
        if new_reachable_items.is_empty() {
            break;
        }
        trace!("#{i}: {new_reachable_items:?}\n");
        reachable_items.extend(new_reachable_items);
    }
    trace!(
        "unreachable items: {:?}",
        info.all_items.difference(&reachable_items).collect_vec()
    );

    reachable_items
}

pub fn list_ambigous_sources(info: &Info) {
    let reachable_items = reachable_items(info);
    for item in &info.all_items {
        let recipes = info
            .game_data
            .recipes
            .values()
            .filter(|r| {
                r.products.iter().any(|p| &p.name == item)
                    && r.ingredients
                        .iter()
                        .all(|ing| reachable_items.contains(&ing.name))
            })
            .collect_vec();
        if recipes.len() > 1 {
            trace!("{item}");
            for recipe in recipes {
                trace!(
                    "- [{}] {}",
                    recipe.name,
                    recipe.ingredients.iter().map(|ing| &ing.name).join(" + ")
                );
            }
            trace!("");
        }
    }
}
