use std::{collections::HashSet, fs, path::Path, path::PathBuf};

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

const ALLOWED_MEDIA_FORMATS: &[&str] = &[
    "mp3", "wav", "flac", "aac", "ogg", "m4a", "mp4", "mkv", "mov", "avi", "webm", "wmv",
];

const TXT_FORMAT: &str = "txt";

fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ALLOWED_MEDIA_FORMATS.contains(&ext.to_lowercase().as_str()))
}

fn is_txt_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(TXT_FORMAT))
}

fn extract_file_stem(path: &Path) -> Option<&str> {
    path.file_stem().and_then(|n| n.to_str())
}

fn list_media_files(dirpath: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let dirpath = dirpath.as_ref();
    log::info!("Searching for media files in {:?}", dirpath);

    let mut res: Vec<PathBuf> = Vec::new();
    let mut txt_files_stems = HashSet::new();

    for entry in fs::read_dir(dirpath)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if is_media_file(&path) {
                res.push(path);
            } else if is_txt_file(&path)
                && let Some(stem) = extract_file_stem(&path)
            {
                txt_files_stems.insert(stem.to_string());
            }
        }
    }

    res.retain(|media_path| {
        !extract_file_stem(media_path).is_some_and(|stem| txt_files_stems.contains(stem))
    });

    Ok(res)
}

fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbosity.into())
        .init();

    let media_files = list_media_files(&args.dirpath)?;
    let media_files_len = media_files.len();
    log::info!("Number of media files to process: {}", media_files_len);

    Ok(())
}
