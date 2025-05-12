use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub furnace_type: String,
    pub assembler_type: String,

    // 1, 2, 3
    #[serde(default = "default_module_tier")]
    pub module_tier: u32,
}

fn default_module_tier() -> u32 {
    1
}
