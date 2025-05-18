use {
    crate::primitives::Amount,
    anyhow::Context,
    serde::{Deserialize, Deserializer, Serialize},
    std::collections::BTreeMap,
};

// Lua doesn't distinguish between empty arrays and empty objects, so empty arrays in game_data.json are serialized as {}.
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
    pub allowed_effects: Effects,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Effects {
    pub consumption: bool,
    pub speed: bool,
    pub productivity: bool,
    pub pollution: bool,
    pub quality: bool,
}

// Skipped "fluidbox_index", "fluidbox_multiplier", "ignored_by_stats" properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ingredient {
    #[serde(rename = "type")]
    pub type_: String, // item or fluid
    pub name: String,
    pub amount: Amount,
}

// Skipped "fluidbox_index", "ignored_by_stats", "percent_spoiled" properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    pub amount: Amount,
    #[serde(default)]
    pub ignored_by_productivity: Amount,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
    pub energy_usage: Option<f64>,
    pub crafting_categories: Option<BTreeMap<String, bool>>,
    pub crafting_speed: Option<f64>,
    pub ingredient_count: Option<u64>,
    pub max_item_product_count: Option<u64>,
    pub mining_speed: Option<f64>,
    pub resource_categories: Option<BTreeMap<String, bool>>,
    pub belt_speed: Option<f64>,
    pub mineable_properties: Option<MineableProperties>,
    pub resource_category: Option<String>,
    pub module_inventory_size: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MineableProperties {
    pub mining_time: f64,
    #[serde(deserialize_with = "deserialize_array_or_object")]
    pub products: Vec<Product>,
    pub fluid_amount: Option<Amount>,
    pub required_fluid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameData {
    pub recipes: BTreeMap<String, Recipe>,
    pub entities: BTreeMap<String, Entity>,
}

impl GameData {
    pub fn recipe(&self, name: &str) -> anyhow::Result<&Recipe> {
        self.recipes
            .get(name)
            .with_context(|| format!("invalid recipe name: {name:?}"))
    }
}
