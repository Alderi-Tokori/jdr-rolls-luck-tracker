#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::Theme;
use iced::theme;

mod db;
mod gui;
mod maths;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database = db::DBHandle::init().await?;

    iced::application(gui::State::default, gui::update, gui::view)
        .style(|_state, theme: &Theme| theme::Style {
            background_color: iced::Color::from_rgb(255.0 / 255.0, 253.0 / 255.0, 232.0 / 255.0),
            text_color: theme.palette().text,
        })
        .run()?;

    Ok(())
}
