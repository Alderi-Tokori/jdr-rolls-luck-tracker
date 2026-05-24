#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod gui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database = db::DBHandle::init().await?;

    iced::run(gui::Counter::update, gui::Counter::view)?;

    Ok(())
}
