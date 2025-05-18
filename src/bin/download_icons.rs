use {
    factories::Info,
    std::{collections::HashMap, path::Path},
};

fn main() -> anyhow::Result<()> {
    let info = Info::load()?;
    let client = reqwest::blocking::Client::new();
    let blacklist = [
        "turbo-loader",
        "express-loader",
        "fast-loader",
        "recipe-unknown",
        "coin",
        "copy-paste-tool",
        "cut-paste-tool",
        "empty-module-slot",
        "infinity-cargo-wagon",
        "item-unknown",
        "proxy-container",
        "science",
        "selection-tool",
        "simple-entity-with-force",
        "simple-entity-with-owner",
    ];
    let renames: HashMap<_, _> = [
        ("battery-equipment", "Personal_battery"),
        ("battery-mk2-equipment", "Personal_battery_MK2"),
        ("battery-mk3-equipment", "Personal_battery_MK3"),
        ("capture-robot-rocket", "Capture_bot_rocket"),
        ("discharge-defense-equipment", "Discharge_defense"),
        (
            "empty-fluoroketone-cold-barrel",
            "Empty_fluoroketone_%28cold%29_barrel",
        ),
        (
            "empty-fluoroketone-hot-barrel",
            "Empty_fluoroketone_%28hot%29_barrel",
        ),
        ("energy-shield-equipment", "Energy_shield"),
        ("energy-shield-mk2-equipment", "Energy_shield_MK2"),
        ("exoskeleton-equipment", "Exoskeleton"),
        ("fission-reactor-equipment", "Portable_fission_reactor"),
        (
            "fluoroketone-cold-barrel",
            "Fill_fluoroketone_%28cold%29_barrel",
        ),
        ("fluoroketone-cooling", "Fluoroketone_%28cold%29"),
        (
            "fluoroketone-hot-barrel",
            "Fill_fluoroketone_%28hot%29_barrel",
        ),
        ("fluoroketone", "Fluoroketone_%28hot%29"),
        ("fusion-reactor-equipment", "Portable_fusion_reactor"),
        ("long-handed-inserter", "Long-handed_inserter"),
        ("night-vision-equipment", "Nightvision"),
        ("personal-laser-defense-equipment", "Personal_laser_defense"),
        ("personal-roboport-equipment", "Personal_roboport"),
        ("personal-roboport-mk2-equipment", "Personal_roboport_MK2"),
        ("piercing-shotgun-shell", "Piercing_shotgun_shells"),
        ("power-armor-mk2", "Power_armor_MK2"),
        ("rail", "Straight_rail"),
        ("shotgun-shell", "Shotgun_shells"),
        ("small-lamp", "Lamp"),
        ("solar-panel-equipment", "Portable_solar_panel"),
        ("space-platform-starter-pack", "Space_platform_hub"),
        ("stone-wall", "Wall"),
        ("teslagun", "Tesla_gun"),
        ("fluoroketone-cold", "Fluoroketone_%28cold%29"),
        ("fluoroketone-hot", "Fluoroketone_%28hot%29"),
        ("uranium-235", "Uranium-235"),
        ("uranium-238", "Uranium-238"),
    ]
    .into_iter()
    .collect();
    for recipe in info
        .game_data
        .recipes
        .keys()
        .map(|n| n.as_str())
        .chain(info.all_items.iter().map(|n| n.as_str()))
    {
        if blacklist.contains(&recipe) {
            continue;
        }
        let file_path = format!("icons/factorio/{recipe}.png");
        if !Path::new(&file_path).exists() {
            println!("downloading {recipe}");
            let mut name = recipe.replace("-", "_");
            name[0..1].make_ascii_uppercase();
            if let Some(n) = renames.get(recipe) {
                name = n.to_string();
            }
            let handle = || {
                let data = client
                    .get(format!(
                        "https://wiki.factorio.com/images/thumb/{n}.png/32px-{n}.png",
                        n = name
                    ))
                    .send()?
                    .error_for_status()?
                    .bytes()?;
                fs_err::write(file_path, data)?;
                anyhow::Ok(())
            };
            if let Err(err) = handle() {
                println!("failed: {err}");
            }
        }
    }
    println!("done");
    Ok(())
}
