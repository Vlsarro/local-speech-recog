# Local speech recognition via Whisper

## Run Whisper web service
This repo makes use of Whisper ASR Webservice (https://ahmetoner.com/whisper-asr-webservice)

### Configuration
```bash
cp env.example .env
```

Then you can update configuration values in `.env`:
```
ASR_MODEL=turbo # tiny, base, small, medium, large-v3, turbo, etc.
ASR_ENGINE=openai_whisper # openai_whisper, faster_whisper, whisperx
WHISPER_SERVICE_IMAGE_TAG=latest # latest, latest-gpu
```

### Run via docker compose
```bash
docker compose -f docker-compose.whisper.yml up -d
```

Selected model will be downloaded on the first run if it's not present in `data/whisper/`.

### Stop service
```bash
docker compose -f docker-compose.whisper.yml down
```

### Call web service
```bash
curl -X POST -H "content-type: multipart/form-data" -F "audio_file=@data/audio/roads_should_be_abolished_geBQNOid_7A.mp3" http://0.0.0.0:9005/asr?output=json
```

## Run client
```bash
$ cargo run -- -h

Whisper ASR Webservice client

Usage: local-speech-recog [OPTIONS]

Options:
      --host <HOST>        Whisper ASR Webservice host [default: http://0.0.0.0:9005]
  -d, --dirpath <DIRPATH>  Directory with audio or video files for transcription [default: .]
  -o, --output <OUTPUT>    Output file format [default: text] [possible values: text, json, vtt, srt, tsv]
  -l, --lang <LANG>        Source language code [default: en]
  -v, --verbose...         Increase logging verbosity
  -q, --quiet...           Decrease logging verbosity
  -h, --help               Print help
  -V, --version            Print version
```

If you have compiled binary then just run it according to usage notes above.
