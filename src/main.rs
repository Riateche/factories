#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(clippy::collapsible_if)]

fn main() -> anyhow::Result<()> {
    factories::ui::run()
}
