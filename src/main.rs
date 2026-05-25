use anyhow::{Ok, Result};
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};

/// Whisper ASR Webservice client
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Whisper ASR Webservice host
    #[arg(long, default_value = "0.0.0.0:9005")]
    host: String,

    /// Directory with audio or video files for transcription
    #[arg(short, long, default_value = ".")]
    dirpath: String,

    /// Output file format, text, json, vtt, srt, tsv
    #[arg(short, long, default_value = "text")]
    output: String,

    /// Source language code
    #[arg(short, long, default_value = "en")]
    lang: String,

    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbosity.into())
        .init();

    log::info!("Args are: {:?}", args);

    Ok(())
}
