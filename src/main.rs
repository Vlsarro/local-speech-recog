use std::fs::File;
use std::io::Write;
use std::{collections::HashSet, fs, path::Path, path::PathBuf};

use anyhow::{Context, Ok, Result};
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};

use reqwest::Url;
use reqwest::blocking::{Client, multipart};

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

const ASR_ENDPOINT: &str = "/asr";

fn transcribe_file(http_client: &Client, path: &Path, args: &Args) -> Result<String> {
    let url = Url::parse_with_params(
        &args.host,
        &[
            ("encode", "true"),
            ("task", "transcribe"),
            ("language", &args.lang),
            ("initial_prompt", ""),
            ("word_timestamps", "false"),
            ("output", &args.output),
        ],
    )?
    .join(ASR_ENDPOINT)?;

    let input_data = fs::read(path)?;

    let form = multipart::Form::new().part(
        "audio_file",
        multipart::Part::bytes(input_data).file_name("afile"),
    );
    let response = http_client
        .post(url)
        .multipart(form)
        .send()?
        .error_for_status()?;
    let result_text = response.text()?;
    Ok(result_text)
}

fn save_file(path: &Path, data: &str) -> Result<String> {
    let txt_filepath = path.with_extension(TXT_FORMAT);
    let txt_filepath_str = txt_filepath.to_string_lossy().into_owned();
    let mut file = File::create(txt_filepath)?;
    file.write_all(data.as_bytes())?;
    Ok(txt_filepath_str)
}

fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbosity.into())
        .init();

    let media_files = list_media_files(&args.dirpath)?;
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
        let result_txt_filename = save_file(filepath, &transcription_data)
            .with_context(|| format!("Failed to save data from processed {:?}", filepath))?;

        log::info!("File transacription was saved to {}", result_txt_filename);
    }

    Ok(())
}
