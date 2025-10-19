use factories::editor::Editor;

fn main() -> anyhow::Result<()> {
    // let recipe = info
    //     .game_data
    //     .recipe(&RecipeName("assembling-machine-3".into()))?;

    let mut editor = Editor::init()?;
    editor.add_crafter(
        &"assembling-machine-3".into(),
        Some(&"assembling-machine-3".into()),
    )?;

    println!("machines {:#?}", editor.machines());
    Ok(())
}
