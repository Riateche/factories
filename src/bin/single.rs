use factories::prelude::*;

fn main() -> Result<()> {
    let p = init()?;
    let m = p.create_machine("transport-belt")?;
    m.print_io();

    Ok(())
}
