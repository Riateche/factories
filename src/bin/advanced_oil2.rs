use factories::prelude::*;

fn main() -> Result<()> {
    let mut p = init()?;
    p.create_machine_with_crafter("plastic-bar", "chemical-plant")?;
    p.create_machine("advanced-oil-processing")?;
    p.create_machine_with_crafter("heavy-oil-cracking", "chemical-plant")?;
    p.create_machine_with_crafter("light-oil-cracking", "chemical-plant")?;
    p.create_machine("solid-fuel-from-light-oil")?;
    p.create_source("coal")?;
    p.create_source("crude-oil")?;
    p.create_source("water")?;
    p.create_sink("plastic-bar")?;
    p.create_sink("solid-fuel")?;
    p.add_item_speed_constraint("plastic-bar", 15.0)?;
    p.add_item_speed_constraint("solid-fuel", 10.0)?;
    p.solve()?;
    p.show_machines();

    Ok(())
}
