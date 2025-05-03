use std::path::PathBuf;
use clap::{Parser, Subcommand};
use clap::builder::ValueParser;

#[derive(Debug, Parser)]
#[command(
    name = "Arcaea Auto Hit Sound",
    author = "Emil Stampfly He",
    version = "0.0.0",
    about = "Generate hit sounds based on the .aff file.",
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(name = "sound")]
    Sound {
        #[arg(
            value_name = "INPUT_PATH",
            value_parser = ValueParser::path_buf()
        )]
        input_path: PathBuf,

        #[arg(
            value_name = "INPUT_PATH",
            value_parser = ValueParser::path_buf()
        )]
        out_path: PathBuf,

        #[arg(
            value_name = "HIT_SOUND_PATH",
            value_parser = ValueParser::path_buf()
        )]
        hit_sound_path: Option<PathBuf>,
    }
}
