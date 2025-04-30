use factories::prelude::*;

fn main() -> Result<()> {
    let mut p = init()?;
    p.create_machine_with_crafter("plastic-bar", "chemical-plant")?;
    p.create_machine("basic-oil-processing")?;
    // p.create_machine("plastic-bar")?;
    // p.create_machine("plastic-bar")?;
    // p.create_machine("plastic-bar")?;
    // p.create_machine("plastic-bar")?;
    // p.create_machine("plastic-bar")?;
    // p.create_machine("plastic-bar")?;
    // p.create_machine("plastic-bar")?;
    // p.create_machine("plastic-bar")?;
    // p.create_machine("plastic-bar")?;
    // p.create_machine("plastic-bar")?;
    // p.create_machine("copper-cable")?;
    p.create_source("coal")?;
    p.create_source("crude-oil")?;
    // p.create_source("copper-plate")?;
    p.create_sink("plastic-bar")?;
    p.add_constraint("plastic-bar", 2.0)?;
    p.solve()?;
    p.show_machines();

    Ok(())
}
