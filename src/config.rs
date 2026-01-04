use {
    crate::{
        machine::ModuleType,
        primitives::{CrafterName, ItemNameAndQuality, Quality},
    },
    serde::{Deserialize, Serialize},
    std::collections::BTreeMap,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub furnace_type: CrafterName,
    pub assembler_type: CrafterName,

    #[serde(default)]
    pub beacon_quality: Quality,
    #[serde(default)]
    pub crafter_qualities: BTreeMap<CrafterName, Quality>,

    #[serde(default)]
    pub modules: BTreeMap<ModuleType, ItemNameAndQuality>,
}
