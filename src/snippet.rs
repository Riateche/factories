use {
    crate::primitives::{CrafterName, ItemName, MachineCount, ModuleName, RecipeName, Speed},
    serde::{Deserialize, Serialize},
    std::collections::BTreeMap,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SnippetMachine {
    Source {
        item: ItemName,
    },
    Sink {
        item: ItemName,
    },
    Crafter {
        crafter: CrafterName,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        modules: Vec<ModuleName>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        beacons: Vec<Vec<ModuleName>>,
        recipe: RecipeName,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        count_constraint: Option<MachineCount>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snippet {
    pub machines: Vec<SnippetMachine>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub item_speed_constraints: BTreeMap<ItemName, Speed>,
}
