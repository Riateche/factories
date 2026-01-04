use {
    crate::primitives::{
        CrafterName, ItemNameAndQuality, MachineCount, ModuleName, Quality, RecipeName, Speed,
    },
    serde::{Deserialize, Serialize},
    std::collections::BTreeMap,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MachineSnippet {
    Source(SourceSinkSnippet),
    Sink(SourceSinkSnippet),
    Crafter(CrafterSnippet),
}

impl From<CrafterSnippet> for MachineSnippet {
    fn from(value: CrafterSnippet) -> Self {
        Self::Crafter(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceSinkSnippet {
    pub item: ItemNameAndQuality,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CrafterSnippet {
    pub crafter: CrafterName,
    #[serde(default, skip_serializing_if = "Quality::is_zero")]
    pub crafter_quality: Quality,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<ItemNameAndQuality>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beacons: Vec<BeaconSnippet>,
    pub recipe: RecipeName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count_constraint: Option<MachineCount>,
    #[serde(default, skip_serializing_if = "Quality::is_zero")]
    pub recipe_quality: Quality,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Snippet {
    pub machines: Vec<MachineSnippet>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub item_speed_constraints: BTreeMap<ItemNameAndQuality, Speed>,
    #[serde(default = "true_")]
    pub auto_add_sources_and_sinks: bool,
    #[serde(default)]
    pub use_alt_solver: bool,
}

fn true_() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct BeaconSnippet {
    pub quality: Quality,
    pub modules: Vec<ModuleName>,
}
