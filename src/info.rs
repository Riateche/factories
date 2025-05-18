use {
    crate::{
        config::Config,
        game_data::GameData,
        machine::{Crafter, Module, ModuleType},
        primitives::{CrafterName, ItemName, ModuleName, RecipeCategory},
    },
    anyhow::{bail, Context},
    std::{
        collections::{BTreeMap, BTreeSet},
        env,
        path::Path,
    },
    tracing::{info, trace},
};

#[derive(Debug)]
pub struct Info {
    pub config: Config,
    pub game_data: GameData,
    pub modules: BTreeMap<ModuleName, Module>,
    pub all_items: BTreeSet<ItemName>,
    pub crafters: BTreeMap<CrafterName, Crafter>,
    pub category_to_crafter: BTreeMap<RecipeCategory, Vec<CrafterName>>,
}

impl Info {
    pub fn load() -> anyhow::Result<Info> {
        if !env::current_dir().unwrap().join("game_data.json").exists() {
            if Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("game_data.json")
                .exists()
            {
                match env::set_current_dir(env!("CARGO_MANIFEST_DIR")) {
                    Ok(()) => info!("changed current dir to {}", env!("CARGO_MANIFEST_DIR")),
                    Err(err) => bail!(
                        "failed to change current dir to {}: {}",
                        env!("CARGO_MANIFEST_DIR"),
                        err
                    ),
                }
            } else {
                bail!("game_data.json not found in working directory");
            }
        }

        let config: Config = toml::from_str(&fs_err::read_to_string("config.toml")?)?;
        let mut game_data: GameData =
            serde_json::from_str(&fs_err::read_to_string("game_data.json")?)?;

        let blacklist = [
            "turbo-loader",
            "express-loader",
            "fast-loader",
            "recipe-unknown",
        ];
        game_data.recipes.retain(|_, recipe| {
            recipe.category != "recycling"
                && recipe.category != "recycling-or-hand-crafting"
                && recipe.category != "captive-spawner-process"
                && recipe.category != "parameters"
                && !blacklist.contains(&recipe.name.as_str())
        });

        let mut all_items = BTreeSet::new();
        for recipe in game_data.recipes.values() {
            for item in &recipe.ingredients {
                all_items.insert(item.name.clone());
            }
            for item in &recipe.products {
                all_items.insert(item.name.clone());
            }
        }

        let mut crafters = BTreeMap::new();
        let mut category_to_crafter = BTreeMap::<_, Vec<_>>::new();
        for entity in game_data.entities.values() {
            if entity.name == "character" {
                continue;
            }
            if let Some(categories) = &entity.crafting_categories {
                for category in categories.keys() {
                    category_to_crafter
                        .entry(category.clone())
                        .or_default()
                        .push(entity.name.as_str().into());
                }
                crafters.insert(
                    entity.name.as_str().into(),
                    Crafter {
                        name: entity.name.as_str().into(),
                        energy_usage: entity.energy_usage.with_context(|| {
                            format!("missing energy_usage for crafter: {entity:?}")
                        })?,
                        crafting_speed: entity.crafting_speed.with_context(|| {
                            format!("missing crafting_speed for crafter: {entity:?}")
                        })?,
                        module_inventory_size: entity.module_inventory_size,
                    },
                );
            }
        }

        for (category, crafters) in &category_to_crafter {
            trace!("{}: {}     {:?}", category, crafters.len(), crafters);
        }

        let modules = [
            Module {
                name: "speed-module".into(),
                type_: ModuleType::Speed,
                energy_delta_percent: 50.,
                speed_delta_percent: 20.,
                productivity_delta_percent: 0.,
            },
            Module {
                name: "speed-module-2".into(),
                type_: ModuleType::Speed,
                energy_delta_percent: 60.,
                speed_delta_percent: 30.,
                productivity_delta_percent: 0.,
            },
            Module {
                name: "speed-module-3".into(),
                type_: ModuleType::Speed,
                energy_delta_percent: 70.,
                speed_delta_percent: 50.,
                productivity_delta_percent: 0.,
            },
            Module {
                name: "productivity-module".into(),
                type_: ModuleType::Productivity,
                energy_delta_percent: 40.,
                speed_delta_percent: -5.,
                productivity_delta_percent: 4.0,
            },
            Module {
                name: "productivity-module-2".into(),
                type_: ModuleType::Productivity,
                energy_delta_percent: 60.,
                speed_delta_percent: -10.,
                productivity_delta_percent: 6.0,
            },
            Module {
                name: "productivity-module-3".into(),
                type_: ModuleType::Productivity,
                energy_delta_percent: 80.,
                speed_delta_percent: -15.,
                productivity_delta_percent: 10.0,
            },
        ]
        .into_iter()
        .map(|m| (m.name.clone(), m))
        .collect();
        Ok(Info {
            config,
            game_data,
            all_items,
            modules,
            crafters,
            category_to_crafter,
        })
    }

    pub fn auto_select_crafter(&self, crafters: &[CrafterName]) -> Option<CrafterName> {
        if crafters.len() == 1 {
            Some(crafters[0].clone())
        } else if crafters.iter().any(|c| c == &self.config.assembler_type) {
            Some(self.config.assembler_type.clone())
        } else if crafters.iter().any(|c| c == &self.config.furnace_type) {
            Some(self.config.furnace_type.clone())
        } else {
            None
        }
    }

    pub fn module(&self, name: &ModuleName) -> anyhow::Result<&Module> {
        self.modules
            .get(name)
            .with_context(|| format!("invalid module name: {name:?}"))
    }
}
