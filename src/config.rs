use {
    crate::primitives::{CrafterName, Quality},
    serde::{Deserialize, Serialize},
    std::collections::BTreeMap,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub furnace_type: CrafterName,
    pub assembler_type: CrafterName,
    #[serde(default)]
    pub crafter_qualities: BTreeMap<CrafterName, Quality>,

    #[serde(default = "default_module_tier")]
    pub speed_module_tier: u32,
    #[serde(default = "default_quality")]
    pub speed_module_quality: Quality,

    #[serde(default = "default_module_tier")]
    pub productivity_module_tier: u32,
    #[serde(default = "default_quality")]
    pub productivity_module_quality: Quality,

    #[serde(default = "default_module_tier")]
    pub quality_module_tier: u32,
    #[serde(default = "default_quality")]
    pub quality_module_quality: Quality,
}

fn default_module_tier() -> u32 {
    1
}

fn default_quality() -> Quality {
    0.into()
}
