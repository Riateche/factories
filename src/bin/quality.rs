use factories::{primitives::RecipeName, Info};

fn main() -> anyhow::Result<()> {
    let info = Info::load()?;
    let recipe = info
        .game_data
        .recipe(&RecipeName("assembling-machine-3".into()))?;
    println!("recipe {recipe:?}");
    Ok(())
}
