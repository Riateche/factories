use std::collections::BTreeMap;

use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};

// Lua doesn't distinguish between empty arrays and empty objects, so empty arrays in recipes.json are serialized as {}.
fn deserialize_array_or_object<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    struct Empty {}

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Untagged<T> {
        Empty(Empty),
        Array(Vec<T>),
    }

    let value = Untagged::<T>::deserialize(deserializer)?;
    match value {
        Untagged::Empty(_) => Ok(Vec::new()),
        Untagged::Array(data) => Ok(data),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub enabled: bool,
    // ["crafting", "pressing", "crafting-with-fluid-or-metallurgy", "metallurgy", "electronics", "smelting", "recycling", "crafting-with-fluid",
    //  "cryogenics", "metallurgy-or-assembling", "organic-or-assembling", "electronics-or-assembling", "cryogenics-or-assembling", "electromagnetics",
    //  "oil-processing", "organic-or-chemistry", "chemistry", "chemistry-or-cryogenics", "electronics-with-fluid", "advanced-crafting",
    //  "rocket-building", "centrifuging", "recycling-or-hand-crafting", "organic-or-hand-crafting", "organic", "captive-spawner-process",
    //  "crushing", "parameters"]
    pub category: String,
    #[serde(deserialize_with = "deserialize_array_or_object")]
    pub ingredients: Vec<Ingredient>,
    #[serde(deserialize_with = "deserialize_array_or_object")]
    pub products: Vec<Product>,
    pub hidden: bool,
    pub hidden_from_flow_stats: bool,
    pub energy: f64,
    // The string used to alphabetically sort these prototypes. It is a simple string that has no additional semantic meaning.
    pub order: String,
    pub productivity_bonus: f64,
}

// Skipped "fluidbox_index", "fluidbox_multiplier", "ignored_by_stats" properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ingredient {
    #[serde(rename = "type")]
    pub type_: String, // item or fluid
    pub name: String,
    pub amount: u64,
}

// Skipped "fluidbox_index", "ignored_by_productivity", "ignored_by_stats", "percent_spoiled" properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    pub amount: f64,
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String, // item or fluid
    /// Probability that a craft will yield one additional product. Also applies to bonus crafts caused by productivity.
    #[serde(default)]
    pub extra_count_fraction: f64,
    /// A value in range [0, 1]. Item is only given with this probability; otherwise no product is produced.
    pub probability: f64,
    #[serde(default)]
    pub temperature: Option<f64>,
}

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

// fn calc(recipes: &mut [Recipe], io: &[&'static str], target: (f32, &'static str)) {
//     loop {
//         let mut speeds = BTreeMap::<&'static str, f32>::new();
//         for recipe in &*recipes {
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

//             for recipe in &mut *recipes {
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
//     for r in recipes {
//         print!("{:.1} x ", r.machines);
//         for i in &r.outputs {
//             print!("{} ", i.item);
//         }
//         println!();
//     }
// }

// #[allow(dead_code)]
// fn smart_plating() {
//     let mut recipes = [
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
//         &mut recipes,
//         &["Iron Ingot", "Copper Ingot", "Smart Plating"],
//         (-120., "Iron Ingot"),
//     );
// }
// #[allow(dead_code)]
// fn auto_wiring() {
//     let mut recipes = [
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
//         &mut recipes,
//         &["Copper Ingot", "Steel Ingot", "Automated Wiring"],
//         (-240., "Copper Ingot"),
//     );
// }
// #[allow(dead_code)]
// fn motors() {
//     let mut recipes = [
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
//         &mut recipes,
//         &["Copper Ingot", "Steel Ingot", "Iron Ingot", "Motor"],
//         (-240., "Iron Ingot"),
//     );
// }
// #[allow(dead_code)]
// fn rotors() {
//     let mut recipes = [
//         Recipe::new(1., &[(20., "Iron Rod"), (100., "Screw")], &[(4., "Rotor")]),
//         Recipe::new(3.1, &[(15., "Iron Ingot")], &[(15., "Iron Rod")]),
//         Recipe::new(2.6, &[(10., "Iron Rod")], &[(40., "Screw")]),
//         Recipe::new(1., &[(30., "Iron Ingot")], &[(20., "Iron Plate")]),
//     ];
//     calc(
//         &mut recipes,
//         &["Iron Ingot", "Rotor"],
//         (-240., "Iron Ingot"),
//     );
// }
fn main() -> anyhow::Result<()> {
    //rotors();
    let recipes: BTreeMap<String, Recipe> =
        serde_json::from_str(&fs_err::read_to_string("recipes.json")?)?;
    let recipes = recipes.into_values().collect_vec();
    println!("{recipes:#?}");

    Ok(())
}

/*


key_values = {}; for (k in v) { if (!Array.isArray(v[k].products)) { continue; };  for(val of v[k].products) { for (kk in val) { if (!key_values[kk]) { key_values[kk] = new Set(); } key_values[kk].push(val[kk]); } } }; console.log(key_values)

key_values = {}; for (k in v) { if (!Array.isArray(v[k].ingredients)) { continue; };  for(val of v[k].ingredients) { for (kk in val) { if (!key_values[kk]) { key_values[kk] = new Set(); } key_values[kk].add(val[kk]); } } }; console.log(key_values)

key_values = {}; for (kk in v) { let item = v[kk]; for (k in item) { if (k == "ingredients" || k == "products") { continue; };  if (!key_values[k]) { key_values[k] = new Set(); } key_values[k].add(item[k]); } }; for (k in key_values) console.log(k, [...key_values[k]])
*/
