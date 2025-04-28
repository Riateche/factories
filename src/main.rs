mod game_data;

use std::collections::BTreeSet;

use game_data::GameData;
use itertools::Itertools;

// struct ItemSpeed {
//     item: &'static str,
//     speed: f32,
// }

// struct Recipe {
//     machines: f32,
//     inputs: Vec<ItemSpeed>,
//     outputs: Vec<ItemSpeed>,
// }

// impl Recipe {
//     fn new(machines: f32, inputs: &[(f32, &'static str)], outputs: &[(f32, &'static str)]) -> Self {
//         Self {
//             machines,
//             inputs: inputs
//                 .iter()
//                 .map(|&(speed, item)| ItemSpeed { speed, item })
//                 .collect(),
//             outputs: outputs
//                 .iter()
//                 .map(|&(speed, item)| ItemSpeed { speed, item })
//                 .collect(),
//         }
//     }
// }

// fn calc(game_data: &mut [Recipe], io: &[&'static str], target: (f32, &'static str)) {
//     loop {
//         let mut speeds = BTreeMap::<&'static str, f32>::new();
//         for recipe in &*game_data {
//             for i in &recipe.inputs {
//                 *speeds.entry(i.item).or_default() -= i.speed * recipe.machines;
//             }
//             for i in &recipe.outputs {
//                 *speeds.entry(i.item).or_default() += i.speed * recipe.machines;
//             }
//         }
//         println!();
//         println!("--- I/O ---");
//         for (item, speed) in &speeds {
//             if io.contains(item) {
//                 println!("{} {}", speed, item);
//             }
//         }
//         println!("--- Intermediate ---");
//         let mut any_bad = false;
//         for (item, speed) in &speeds {
//             let delta;
//             if io.contains(item) {
//                 if *item == target.1 {
//                     delta = target.0 - speed;
//                 } else {
//                     continue;
//                 }
//             } else {
//                 println!("{} {}", speed, item);
//                 delta = 0. - speed;
//             }
//             if delta.abs() < 0.01 {
//                 continue;
//             }
//             any_bad = true;
//             let d = 0.1 * delta.abs();

//             for recipe in &mut *game_data {
//                 if let Some(i) = recipe.inputs.iter().find(|x| x.item == *item) {
//                     if delta < 0. {
//                         recipe.machines += d / i.speed;
//                     } else {
//                         recipe.machines -= d / i.speed;
//                     }
//                 }
//                 if let Some(i) = recipe.outputs.iter().find(|x| x.item == *item) {
//                     if delta < 0. {
//                         recipe.machines -= d / i.speed;
//                     } else {
//                         recipe.machines += d / i.speed;
//                     }
//                 }
//             }
//         }
//         if !any_bad {
//             break;
//         }
//     }
//     println!("--- Final machines ---");
//     for r in game_data {
//         print!("{:.1} x ", r.machines);
//         for i in &r.outputs {
//             print!("{} ", i.item);
//         }
//         println!();
//     }
// }

// #[allow(dead_code)]
// fn smart_plating() {
//     let mut game_data = [
//         Recipe::new(
//             1.,
//             &[(2., "Reinforced Iron Plate"), (2., "Rotor")],
//             &[(2., "Smart Plating")],
//         ),
//         Recipe::new(
//             1.,
//             &[(30., "Iron Plate"), (60., "Screw")],
//             &[(5., "Reinforced Iron Plate")],
//         ),
//         Recipe::new(1., &[(20., "Iron Rod"), (100., "Screw")], &[(4., "Rotor")]),
//         Recipe::new(3.1, &[(15., "Iron Ingot")], &[(15., "Iron Rod")]),
//         Recipe::new(2.6, &[(10., "Iron Rod")], &[(40., "Screw")]),
//         Recipe::new(1., &[(30., "Iron Ingot")], &[(20., "Iron Plate")]),
//     ];
//     calc(
//         &mut game_data,
//         &["Iron Ingot", "Copper Ingot", "Smart Plating"],
//         (-120., "Iron Ingot"),
//     );
// }
// #[allow(dead_code)]
// fn auto_wiring() {
//     let mut game_data = [
//         Recipe::new(
//             1.,
//             &[(2.5, "Stator"), (50., "Cable")],
//             &[(2.5, "Automated Wiring")],
//         ),
//         Recipe::new(1., &[(15., "Steel Pipe"), (40., "Wire")], &[(5., "Stator")]),
//         Recipe::new(1., &[(60., "Wire")], &[(30., "Cable")]),
//         Recipe::new(1., &[(15., "Copper Ingot")], &[(30., "Wire")]),
//         Recipe::new(1., &[(30., "Steel Ingot")], &[(20., "Steel Pipe")]),
//         // Recipe::new(1., &[(30., "Iron Ingot")], &[(20., "Iron Plate")]),
//     ];
//     calc(
//         &mut game_data,
//         &["Copper Ingot", "Steel Ingot", "Automated Wiring"],
//         (-240., "Copper Ingot"),
//     );
// }
// #[allow(dead_code)]
// fn motors() {
//     let mut game_data = [
//         Recipe::new(1., &[(10., "Rotor"), (10., "Stator")], &[(5., "Motor")]),
//         Recipe::new(1., &[(15., "Steel Pipe"), (40., "Wire")], &[(5., "Stator")]),
//         Recipe::new(1., &[(15., "Copper Ingot")], &[(30., "Wire")]),
//         Recipe::new(1., &[(30., "Steel Ingot")], &[(20., "Steel Pipe")]),
//         // Recipe::new(1., &[(30., "Iron Ingot")], &[(20., "Iron Plate")]),
//         Recipe::new(1., &[(20., "Iron Rod"), (100., "Screw")], &[(4., "Rotor")]),
//         Recipe::new(3.1, &[(15., "Iron Ingot")], &[(15., "Iron Rod")]),
//         Recipe::new(2.6, &[(10., "Iron Rod")], &[(40., "Screw")]),
//         Recipe::new(1., &[(30., "Iron Ingot")], &[(20., "Iron Plate")]),
//     ];
//     calc(
//         &mut game_data,
//         &["Copper Ingot", "Steel Ingot", "Iron Ingot", "Motor"],
//         (-240., "Iron Ingot"),
//     );
// }
// #[allow(dead_code)]
// fn rotors() {
//     let mut game_data = [
//         Recipe::new(1., &[(20., "Iron Rod"), (100., "Screw")], &[(4., "Rotor")]),
//         Recipe::new(3.1, &[(15., "Iron Ingot")], &[(15., "Iron Rod")]),
//         Recipe::new(2.6, &[(10., "Iron Rod")], &[(40., "Screw")]),
//         Recipe::new(1., &[(30., "Iron Ingot")], &[(20., "Iron Plate")]),
//     ];
//     calc(
//         &mut game_data,
//         &["Iron Ingot", "Rotor"],
//         (-240., "Iron Ingot"),
//     );
// }
fn main() -> anyhow::Result<()> {
    //rotors();
    let game_data: GameData = serde_json::from_str(&fs_err::read_to_string("game_data.json")?)?;
    // println!("{game_data:#?}");

    // let mut set = BTreeSet::new();
    // for entity in game_data.entities.values() {
    //     set.insert(&entity.type_);
    // }
    // println!("{set:?}");

    let mut all_items = BTreeSet::new();
    for recipe in game_data.recipes.values() {
        for item in &recipe.ingredients {
            all_items.insert(item.name.clone());
        }
        for item in &recipe.products {
            all_items.insert(item.name.clone());
        }
    }

    let mut all_reachable_items: BTreeSet<String> = game_data
        .entities
        .values()
        .filter(|v| v.resource_category.is_some() || v.type_ == "plant" || v.type_ == "tree")
        .flat_map(|v| &v.mineable_properties.as_ref().unwrap().products)
        .map(|v| v.name.clone())
        .collect();
    for s in ["water", "lava", "heavy-oil", "ammoniacal-solution"] {
        all_reachable_items.insert(s.into());
    }
    if false {
        println!("all_reachable_items #0: {all_reachable_items:?}\n");
    }

    let mut reachable_items: BTreeSet<String> = [
        "coal",
        "copper-ore",
        "crude-oil",
        "iron-ore",
        "stone",
        "water",
        "wood",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect();

    let mut verified_recipes = BTreeSet::new();

    let mut i = 0;
    loop {
        i += 1;
        let mut new_reachable_items = BTreeSet::new();

        for recipe in game_data.recipes.values() {
            if recipe
                .ingredients
                .iter()
                .all(|ing| reachable_items.contains(&ing.name))
                && recipe.category != "recycling"
                && recipe.category != "recycling-or-hand-crafting"
                && recipe.category != "captive-spawner-process"
            {
                for product in &recipe.products {
                    if !reachable_items.contains(&product.name) {
                        if false {
                            println!(
                                "{} | {} -> {}",
                                recipe.name,
                                recipe.ingredients.iter().map(|ing| &ing.name).join(", "),
                                product.name
                            );
                        }
                        new_reachable_items.insert(product.name.clone());
                    }
                }
                if !verified_recipes.contains(&recipe.name) {
                    if let Some(bad_product) = recipe
                        .products
                        .iter()
                        .find(|product| reachable_items.contains(&product.name))
                    {
                        if false {
                            println!("loop detected for {}: {recipe:?}\n\n", bad_product.name);
                        }
                        // println!("verified_recipes {verified_recipes:?}");
                        // println!("reachable_items {reachable_items:?}\n\n");
                        // if recipe.name == "copper-plate" {
                        //     std::process::exit(1);
                        // }
                    }
                    verified_recipes.insert(recipe.name.to_string());
                }
            }
        }
        if new_reachable_items.is_empty() {
            break;
        }
        if false {
            println!("#{i}: {new_reachable_items:?}\n");
        }
        reachable_items.extend(new_reachable_items);
    }
    if false {
        println!(
            "unreachable items: {:?}",
            all_items.difference(&reachable_items).collect_vec()
        );
    }

    for item in &all_items {
        let recipes = game_data
            .recipes
            .values()
            .filter(|r| {
                r.category != "recycling"
                    && r.category != "recycling-or-hand-crafting"
                    && r.products.iter().any(|p| &p.name == item)
                    && r.ingredients
                        .iter()
                        .all(|ing| reachable_items.contains(&ing.name))
            })
            .collect_vec();
        if recipes.len() > 1 {
            println!("{item}");
            for recipe in recipes {
                println!(
                    "- [{}] {}",
                    recipe.name,
                    recipe.ingredients.iter().map(|ing| &ing.name).join(" + ")
                );
            }
            println!();
        }
    }

    Ok(())
}

/*


key_values = {}; for (k in v) { if (!Array.isArray(v[k].products)) { continue; };  for(val of v[k].products) { for (kk in val) { if (!key_values[kk]) { key_values[kk] = new Set(); } key_values[kk].push(val[kk]); } } }; console.log(key_values)

key_values = {}; for (k in v) { if (!Array.isArray(v[k].ingredients)) { continue; };  for(val of v[k].ingredients) { for (kk in val) { if (!key_values[kk]) { key_values[kk] = new Set(); } key_values[kk].add(val[kk]); } } }; console.log(key_values)

key_values = {}; for (kk in v) { let item = v[kk]; for (k in item) { if (k == "ingredients" || k == "products") { continue; };  if (!key_values[k]) { key_values[k] = new Set(); } key_values[k].add(item[k]); } }; for (k in key_values) console.log(k, [...key_values[k]])
*/
