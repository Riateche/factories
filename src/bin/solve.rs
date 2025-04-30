use factories::prelude::*;

fn main() -> Result<()> {
    let mut p = init()?;
    p.create_machine("electronic-circuit")?;
    p.create_machine("copper-cable")?;
    p.create_source("iron-plate")?;
    p.create_source("copper-plate")?;
    p.create_sink("electronic-circuit")?;
    p.add_constraint("electronic-circuit", 3.0)?;
    p.solve()?;
    p.show_machines();

    Ok(())
}
