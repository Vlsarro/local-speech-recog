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
