use {
    crate::{editor::Editor, primitives::Speed, rf, snippet::SnippetMachine},
    itertools::Itertools,
    std::{cmp::min, collections::VecDeque, fmt::Write},
    tracing::warn,
};

pub fn generate(editor: &Editor, title: &str) -> String {
    let mut out = String::new();
    if !title.is_empty() {
        writeln!(
            out,
            "--- \n\
            title: {}\n\
            ---",
            title
        )
        .unwrap();
    }
    writeln!(out, "flowchart TD").unwrap();
    for (index, editor_machine) in editor.machines().iter().enumerate() {
        let machine = editor_machine.machine();
        let (left_bracket, right_bracket) = match editor_machine.snippet() {
            SnippetMachine::Source { .. } => ("[\\", "/]"),
            SnippetMachine::Sink { .. } => ("[/", "\\]"),
            SnippetMachine::Crafter { .. } => ("([", "])"),
        };

        writeln!(
            out,
            r#"    machine{}{}"{}*{}*(*{}*)"{}"#,
            index,
            left_bracket,
            if machine.crafter.is_source_or_sink() {
                String::new()
            } else {
                format!("{} × ", rf(machine.crafter_count))
            },
            machine.crafter.name,
            if machine.crafter.is_source_or_sink() {
                machine
                    .recipe
                    .ingredients
                    .get(0)
                    .map(|i| &i.name)
                    .unwrap_or_else(|| {
                        machine
                            .recipe
                            .products
                            .get(0)
                            .map(|i| &i.name)
                            .expect("invalid source or sink recipe")
                    })
                    .as_str()
            } else {
                machine.recipe.name.as_str()
            },
            right_bracket,
        )
        .unwrap();
    }

    let all_items = editor.added_items();
    for item in all_items {
        let sources = editor
            .machines()
            .iter()
            .enumerate()
            .filter_map(|(machine_index, machine)| {
                machine
                    .machine()
                    .item_speeds()
                    .into_iter()
                    .find(|item_speed| item_speed.item == item && item_speed.speed > Speed::ZERO)
                    .map(|item_speed| (machine_index, item_speed.speed))
            })
            .collect_vec();

        let mut destinations: VecDeque<_> = editor
            .machines()
            .iter()
            .enumerate()
            .filter_map(|(machine_index, machine)| {
                machine
                    .machine()
                    .item_speeds()
                    .into_iter()
                    .find(|item_speed| item_speed.item == item && item_speed.speed < Speed::ZERO)
                    .map(|item_speed| (machine_index, -item_speed.speed))
            })
            .collect();

        let epsilon = Speed::from(0.001);
        'outer: for (source_machine, source_speed) in sources {
            let mut remaining_speed = source_speed;
            loop {
                let Some((destination_machine, destination_speed)) = destinations.front_mut()
                else {
                    warn!(
                        "unable to allocate remaining {}/s {} to destinations",
                        remaining_speed, item
                    );
                    break 'outer;
                };
                let current_speed = min(remaining_speed, *destination_speed);
                writeln!(
                    out,
                    "    machine{}-->|{} *{}*|machine{}",
                    source_machine, current_speed, item, destination_machine
                )
                .unwrap();
                *destination_speed -= current_speed;
                if *destination_speed < epsilon {
                    destinations.pop_front().unwrap();
                }
                remaining_speed -= current_speed;
                if remaining_speed < epsilon {
                    break; // Move on to the next source machine.
                }
            }
        }
        if destinations.len() > 2
            || destinations
                .front()
                .is_some_and(|(_, speed)| *speed > epsilon)
        {
            warn!(
                "not all destinations of {} are satisfied: {:?}",
                item, destinations
            );
        }
    }
    out
}
