#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use crate::maths::dice::DiceRoll;

mod db;
mod gui;
mod maths;

/// Program allowing the calculation of "luck rating", which is the percentage of people
/// you are luckier than
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Calculate the probability distribution of a given roll
    #[arg(short, long)]
    roll: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let database = db::DBHandle::init().await?;

    if let Some(roll) = args.roll {
        let dice_roll = DiceRoll::parse(& roll);

        return Ok(());
    }

    iced::application(gui::State::default, gui::update, gui::view)
        .style(|_state, theme: &iced::Theme| iced::theme::Style {
            background_color: iced::Color::from_rgb(255.0 / 255.0, 253.0 / 255.0, 232.0 / 255.0),
            text_color: theme.palette().text,
        })
        .run()?;

    Ok(())
}
