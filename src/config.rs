use serde::{Deserialize, Serialize};

use crate::primitives::CrafterName;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub furnace_type: CrafterName,
    pub assembler_type: CrafterName,

    // 1, 2, 3
    #[serde(default = "default_module_tier")]
    pub module_tier: u32,
}

fn default_module_tier() -> u32 {
    1
}
