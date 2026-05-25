use std::fs::File;
use std::io::Write;
use std::{collections::HashSet, fs, path::Path, path::PathBuf};

use anyhow::{Context, Ok, Result};
use clap::{Parser, ValueEnum};
use clap_verbosity_flag::{InfoLevel, Verbosity};

use reqwest::Url;
use reqwest::blocking::{Client, multipart};

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Output {
    Text,
    Json,
    Vtt,
    Srt,
    Tsv,
}

impl Output {
    fn as_str(&self) -> &'static str {
        match self {
            Output::Text => "txt",
            Output::Json => "json",
            Output::Vtt => "vtt",
            Output::Srt => "srt",
            Output::Tsv => "tsv",
        }
    }
}

/// Whisper ASR Webservice client
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Whisper ASR Webservice host
    #[arg(long, default_value = "http://0.0.0.0:9005")]
    host: String,

    /// Directory with audio or video files for transcription
    #[arg(short, long, default_value = ".")]
    dirpath: String,

    /// Output file format
    #[arg(short, long, value_enum, default_value_t = Output::Text)]
    output: Output,

    /// Source language code
    #[arg(short, long, default_value = "en")]
    lang: String,

    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,
}

const ALLOWED_MEDIA_FORMATS: &[&str] = &[
    "mp3", "wav", "flac", "aac", "ogg", "m4a", "mp4", "mkv", "mov", "avi", "webm", "wmv",
];

fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ALLOWED_MEDIA_FORMATS.contains(&ext.to_lowercase().as_str()))
}

fn is_file_already_processed(path: &Path, out_format: &Output) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(out_format.as_str()))
}

fn extract_file_stem(path: &Path) -> Option<&str> {
    path.file_stem().and_then(|n| n.to_str())
}

fn list_media_files(dirpath: impl AsRef<Path>, out_format: &Output) -> Result<Vec<PathBuf>> {
    let dirpath = dirpath.as_ref();
    log::info!("Searching for media files in {:?}", dirpath);

    let mut res: Vec<PathBuf> = Vec::new();
    let mut processed_files_stems = HashSet::new();

    for entry in fs::read_dir(dirpath)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if is_media_file(&path) {
                res.push(path);
            } else if is_file_already_processed(&path, out_format)
                && let Some(stem) = extract_file_stem(&path)
            {
                processed_files_stems.insert(stem.to_string());
            }
        }
    }

    res.retain(|media_path| {
        !extract_file_stem(media_path).is_some_and(|stem| processed_files_stems.contains(stem))
    });

    Ok(res)
}

const ASR_ENDPOINT: &str = "/asr";

fn transcribe_file(http_client: &Client, path: &Path, args: &Args) -> Result<String> {
    let url = Url::parse(&args.host)?.join(ASR_ENDPOINT)?;

    let params = [
        ("encode", "true"),
        ("task", "transcribe"),
        ("language", &args.lang),
        ("initial_prompt", ""),
        ("word_timestamps", "false"),
        ("output", args.output.as_str()),
    ];

    let input_data = fs::read(path)?;

    let form = multipart::Form::new().part(
        "audio_file",
        multipart::Part::bytes(input_data).file_name("afile"),
    );
    let response = http_client
        .post(url)
        .query(&params)
        .multipart(form)
        .send()?
        .error_for_status()?;
    let result_text = response.text()?;
    Ok(result_text)
}

fn save_file(path: &Path, data: &str, out_format: &Output) -> Result<String> {
    let out_filepath = path.with_extension(out_format.as_str());
    let out_filepath_str = out_filepath.to_string_lossy().into_owned();
    let mut file = File::create(out_filepath)?;
    file.write_all(data.as_bytes())?;
    Ok(out_filepath_str)
}

fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbosity.into())
        .init();

    let media_files = list_media_files(&args.dirpath, &args.output)?;
    let media_files_len = media_files.len();
    log::info!("Number of media files to process: {}", media_files_len);

    let http_client = Client::new();
    for (idx, filepath) in media_files.iter().enumerate() {
        log::info!(
            "[{}/{}] Processing {:?} ...",
            idx + 1,
            media_files_len,
            filepath
        );

        let transcription_data = transcribe_file(&http_client, filepath, &args)
            .with_context(|| format!("Failed to process {:?}", filepath))?;
        let result_out_filename = save_file(filepath, &transcription_data, &args.output)
            .with_context(|| format!("Failed to save data from processed {:?}", filepath))?;

        log::info!("File transcription was saved to {}", result_out_filename);
    }

    Ok(())
}
