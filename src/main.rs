#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::anyhow;
use clap::Parser;

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
        let Some(dice_roll) = maths::dice::DiceRoll::parse(& roll) else {
            return Err(anyhow!("Invalid roll query"));
        };

        let distribution = maths::dice::get_dice_roll_distribution(& dice_roll);

        println!("{}", serde_json::to_string(& distribution).unwrap_or("".to_string()));

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
