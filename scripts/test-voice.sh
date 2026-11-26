#!/usr/bin/env bash
# Stub engines exercise orchestration and failure paths without models or devices.
set -euo pipefail
cd "$(dirname "$0")/.."
umask 077
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
touch "$tmp/whisper.bin" "$tmp/voice.onnx" "$tmp/voice.onnx.json"
printf 'fixture' >"$tmp/input.wav"

cat >"$tmp/whisper" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
case "${TEST_ASR_MODE:-ok}" in
  ok) printf 'what is your name\n' ;;
  empty) : ;;
  long) printf '%8195s' 'x' ;;
  control) printf 'hi\033[2J\n' ;;
  fail) exit 9 ;;
esac
STUB
cat >"$tmp/piper" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == -m && "$3" == -f ]]
[[ "$(cat)" == 'I am AtomOS.' ]]
[[ "${TEST_TTS_MODE:-ok}" == ok ]] || exit 8
printf 'stub wave output' >"$4"
STUB
cat >"$tmp/player" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == -q && "$2" == -D && -s "$4" ]]
printf 'played\n' >"$PLAY_MARKER"
STUB
chmod +x "$tmp/whisper" "$tmp/piper" "$tmp/player"
export WHISPER_BIN="$tmp/whisper" WHISPER_MODEL="$tmp/whisper.bin"
export PIPER_BIN="$tmp/piper" PIPER_MODEL="$tmp/voice.onnx"
export APLAY_BIN="$tmp/player" PLAY_MARKER="$tmp/played"
export ATOMOS_BIN=target/debug/atomos

bash scripts/voice-smoke.sh --fixture "$tmp/input.wav" --no-playback
[[ ! -e "$PLAY_MARKER" ]]
bash scripts/voice-smoke.sh --fixture "$tmp/input.wav"
[[ -s "$PLAY_MARKER" ]]
rm -- "$PLAY_MARKER"
for mode in empty long control fail; do
  if TEST_ASR_MODE="$mode" bash scripts/voice-smoke.sh --fixture "$tmp/input.wav"; then
    echo "Unexpected success in ASR case $mode" >&2; exit 1
  fi
  [[ ! -e "$PLAY_MARKER" ]]
done
if TEST_TTS_MODE=fail bash scripts/voice-smoke.sh --fixture "$tmp/input.wav"; then
  echo 'Unexpected success in TTS failure case' >&2; exit 1
fi
[[ ! -e "$PLAY_MARKER" ]]
if bash scripts/voice-smoke.sh --fixture "$tmp/missing.wav"; then
  echo 'Unexpected success with missing fixture' >&2; exit 1
fi
echo 'PASS: stub audio orchestration, no-playback, input bounds and fail-closed paths'
