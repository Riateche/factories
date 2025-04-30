use factories::prelude::*;

fn main() -> Result<()> {
    let mut p = init()?;
    p.create_machine("transport-belt")?;
    p.show_machines();

    Ok(())
}
