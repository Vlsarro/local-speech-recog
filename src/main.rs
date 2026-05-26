use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};
use std::{collections::HashSet, fs, path::Path, path::PathBuf};

use anyhow::{Context, Ok, Result};
use clap::{Parser, ValueEnum};
use clap_verbosity_flag::{InfoLevel, Verbosity};

use reqwest::Url;
use reqwest::blocking::{Client, multipart};

use bytes::Bytes;

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
    #[arg(short, long)]
    lang: Option<String>,

    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,

    /// ASR service response timeout
    #[arg(short, long, value_parser = humantime::parse_duration, default_value = "5m")]
    timeout: Duration,
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

fn transcribe_file(http_client: &Client, filepath: &Path, args: &Args) -> Result<Bytes> {
    let url = Url::parse(&args.host)?.join(ASR_ENDPOINT)?;

    let mut params = vec![
        ("encode", "true"),
        ("task", "transcribe"),
        ("initial_prompt", ""),
        ("word_timestamps", "false"),
        ("output", args.output.as_str()),
    ];

    if let Some(lang) = &args.lang {
        params.push(("language", lang));
    }

    let input_data = fs::read(filepath)?;

    let form = multipart::Form::new().part(
        "audio_file",
        multipart::Part::bytes(input_data).file_name("afile"),
    );

    let response = http_client
        .post(url)
        .query(&params)
        .multipart(form)
        .timeout(args.timeout)
        .send()?
        .error_for_status()?;
    Ok(response.bytes()?)
}

fn save_file(path: &Path, data: &Bytes, out_format: &Output) -> Result<String> {
    let out_filepath = path.with_extension(out_format.as_str());
    let out_filepath_str = out_filepath.to_string_lossy().into_owned();
    let mut file = File::create(out_filepath)?;
    file.write_all(data)?;
    Ok(out_filepath_str)
}

fn main() -> Result<()> {
    let total_processing_start = Instant::now();
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbosity.into())
        .init();

    let media_files = list_media_files(&args.dirpath, &args.output)
        .with_context(|| format!("Failed to list media files in {:?}", args.dirpath))?;
    let media_files_len = media_files.len();
    log::info!("Number of media files to process: {}", media_files_len);

    let http_client = Client::new();
    for (idx, filepath) in media_files.iter().enumerate() {
        let file_processing_start = Instant::now();
        log::info!(
            "[{}/{}] Processing {:?} ...",
            idx + 1,
            media_files_len,
            filepath
        );

        let transcribed_data = match transcribe_file(&http_client, filepath, &args) {
            Result::Ok(data) => data,
            Err(err) => {
                log::error!("Failed to process {:?}: {:?}", filepath, err);
                continue;
            }
        };

        let result_out_filename = match save_file(filepath, &transcribed_data, &args.output) {
            Result::Ok(res) => res,
            Err(err) => {
                log::error!(
                    "Failed to save data from processed {:?}: {:?}",
                    filepath,
                    err
                );
                continue;
            }
        };

        log::info!(
            "File transcription was saved to {}, processing time: {:?}",
            result_out_filename,
            file_processing_start.elapsed()
        );
    }

    log::info!(
        "Files processing finished, total processing time: {:?}",
        total_processing_start.elapsed()
    );
    Ok(())
}
