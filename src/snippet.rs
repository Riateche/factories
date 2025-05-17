use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SnippetMachine {
    Source {
        item: String,
    },
    Sink {
        item: String,
    },
    Crafter {
        crafter: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        modules: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        beacons: Vec<Vec<String>>,
        recipe: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        count_constraint: Option<f64>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snippet {
    pub machines: Vec<SnippetMachine>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub item_speed_constraints: BTreeMap<String, f64>,
}
