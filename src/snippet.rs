use {
    crate::primitives::{CrafterName, ItemName, MachineCount, ModuleName, RecipeName, Speed},
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceSinkSnippet {
    pub item: ItemName,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CrafterSnippet {
    pub crafter: CrafterName,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<ModuleName>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beacons: Vec<BeaconSnippet>,
    pub recipe: RecipeName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count_constraint: Option<MachineCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Snippet {
    pub machines: Vec<MachineSnippet>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub item_speed_constraints: BTreeMap<ItemName, Speed>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BeaconSnippet {
    pub modules: Vec<ModuleName>,
}
