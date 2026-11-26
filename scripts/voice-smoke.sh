#!/usr/bin/env bash
# Explicit single-turn local audio spike. Never starts a continuous listener.
set -euo pipefail
umask 077
cd "$(dirname "$0")/.."

usage() {
  echo 'Usage: voice-smoke.sh (--record | --fixture WAV) [--no-playback]' >&2
  exit 2
}

mode= fixture= play=1
while (($#)); do
  case "$1" in
    --record) [[ -z "$mode" ]] || usage; mode=record; shift ;;
    --fixture) [[ -z "$mode" && $# -ge 2 ]] || usage; mode=fixture; fixture="$2"; shift 2 ;;
    --no-playback) play=0; shift ;;
    *) usage ;;
  esac
done
[[ -n "$mode" ]] || usage

: "${WHISPER_MODEL:?Set WHISPER_MODEL to an installed local ggml model}"
: "${PIPER_MODEL:?Set PIPER_MODEL to an installed local ONNX voice}"
whisper_model="$(realpath -e -- "$WHISPER_MODEL")"
piper_model="$(realpath -e -- "$PIPER_MODEL")"
[[ -f "$whisper_model" && -f "$piper_model" && -f "$piper_model.json" ]] || {
  echo 'Local model files and Piper model JSON required; no automatic downloads.' >&2; exit 2;
}
whisper="${WHISPER_BIN:-whisper-cli}"
piper="${PIPER_BIN:-piper}"
atomos="${ATOMOS_BIN:-target/release/atomos}"
capture="${ARECORD_BIN:-arecord}"
player="${APLAY_BIN:-aplay}"
for executable in "$whisper" "$piper" "$atomos" timeout stat realpath; do
  command -v -- "$executable" >/dev/null || { echo "Missing executable: $executable" >&2; exit 2; }
done
if [[ "$mode" == record ]]; then command -v -- "$capture" >/dev/null || exit 2; fi
if ((play)); then command -v -- "$player" >/dev/null || exit 2; fi

# Private, volatile scratch only. tmpfs may swap on some hosts; see docs.
[[ "$(stat -f -c %T /dev/shm)" == tmpfs ]] || { echo 'tmpfs /dev/shm required' >&2; exit 2; }
work="$(mktemp -d /dev/shm/atomos-voice.XXXXXXXX)"
active=
cleanup() {
  if [[ -n "$active" ]]; then kill -TERM "$active" 2>/dev/null || true; wait "$active" 2>/dev/null || true; fi
  rm -rf -- "$work"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
# Bound output files of every child (Linux bash: 1024-byte blocks).
ulimit -f 8192

run_stage() {
  local stage="$1"; shift
  local start=$SECONDS
  timeout --kill-after=2s 60s "$@" &
  active=$!
  local status=0
  wait "$active" || status=$?
  active=
  printf '%s elapsed_seconds_coarse=%s status=%s\n' "$stage" "$((SECONDS-start))" "$status" >&2
  return "$status"
}

if [[ "$mode" == record ]]; then
  seconds="${RECORD_SECONDS:-5}"
  [[ "$seconds" =~ ^([1-9]|10)$ ]] || { echo 'RECORD_SECONDS must be 1–10' >&2; exit 2; }
  echo "Recording locally for $seconds seconds. Ctrl+C cancels." >&2
  run_stage capture "$capture" -q -D "${CAPTURE_DEVICE:-default}" -t wav -f S16_LE -r 16000 -c 1 -d "$seconds" "$work/input.wav" </dev/null
  input="$work/input.wav"
else
  input="$(realpath -e -- "$fixture")"
fi
[[ -f "$input" && -s "$input" ]] || { echo 'Missing or empty input WAV' >&2; exit 2; }
(( $(stat -c %s -- "$input") <= 2097152 )) || { echo 'Input WAV exceeds 2 MiB' >&2; exit 2; }

# Keep transcripts off the terminal and out of persisted logs.
run_stage asr "$whisper" -m "$whisper_model" -f "$input" -l en -nt -np >"$work/transcript" 2>"$work/asr.log" || {
  echo 'ASR failed; verify local model and executable.' >&2; exit 1;
}
[[ -s "$work/transcript" ]] || { echo 'No transcript; no response.' >&2; exit 1; }
(( $(stat -c %s -- "$work/transcript") <= 8194 )) || { echo 'Transcript exceeds bound' >&2; exit 1; }
run_stage agent "$atomos" --single-turn <"$work/transcript" >"$work/response" 2>"$work/agent.log" || {
  echo 'Agent rejected transcript.' >&2; exit 1;
}
[[ -s "$work/response" ]] || { echo 'Empty agent response' >&2; exit 1; }

run_stage tts "$piper" -m "$piper_model" -f "$work/output.wav" <"$work/response" >"$work/tts.log" 2>&1 || {
  echo 'TTS failed; verify local model and executable.' >&2; exit 1;
}
[[ -s "$work/output.wav" ]] || { echo 'TTS did not produce audio' >&2; exit 1; }
if ((play)); then
  echo 'Playing local response. Ctrl+C cancels playback.' >&2
  run_stage playback "$player" -q -D "${PLAYBACK_DEVICE:-default}" "$work/output.wav" </dev/null
fi
echo 'PASS: local single-turn audio pipeline (mock reasoning; no remote inference by AtomOS)'
