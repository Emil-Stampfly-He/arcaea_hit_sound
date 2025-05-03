use std::path::PathBuf;
use clap::Parser;
use arcaea_auto_hitsound::cli::{Cli, Command};
use arcaea_auto_hitsound::output::output;

fn main() {
    let cli = Cli::parse();

    if let Command::Sound { input_path, out_path, hit_sound_path } = cli.cmd {
        let sound_path = hit_sound_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("hit_sound_sky.wav"));
        if let Err(e) = output(input_path, out_path, sound_path) {
            eprintln!("Error generating sound: {}", e);
            std::process::exit(1);
        }
    }
}
