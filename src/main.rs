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

    /// use GMP to do arbitrary precision calculations
    #[arg(long, default_value_t = false)]
    gmp: bool,

    /// Declare the format you want for distribution output
    #[arg(
        short,
        long,
        value_parser = clap::builder::PossibleValuesParser::new(
            ["json", "csv"]
        ),
        default_value = "json"
    )]
    format: String,

    /// Output the time taken to calculate the distribution
    #[arg(long, default_value_t = false)]
    benchmark: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let database = db::DBHandle::init().await?;

    if let Some(roll) = args.roll {
        let Some(dice_roll) = maths::dice::DiceRoll::parse(& roll) else {
            return Err(anyhow!("Invalid roll query"));
        };

        if ! args.gmp {
            let start = std::time::Instant::now();
            let distribution = maths::dice::get_dice_roll_distribution(&dice_roll);
            let elapsed = start.elapsed();

            match args.format.as_str() {
                "json" => println!("{}", distribution.format_json()),
                "csv" => println!("{}", distribution.format_csv().unwrap_or("".to_string())),
                _ => println!("Format {} not yet implemented!", args.format),
            }

            if args.benchmark {
                println!("Distribution calculated in {:.3?}", elapsed);
            }
        } else {
            let start = std::time::Instant::now();
            let distribution = maths::dice::get_dice_roll_distribution_rational(&dice_roll);
            let elapsed = start.elapsed();

            match args.format.as_str() {
                "json" => println!("{}", distribution.format_json()),
                "csv" => println!("{}", distribution.format_csv().unwrap_or("".to_string())),
                _ => println!("Format {} not yet implemented!", args.format),
            }

            if args.benchmark {
                println!("Distribution calculated in {:.3?}", elapsed);
            }
        }

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
