#!/bin/sh
set -eu
: "${DISPLAY:?Start with xvfb-run}"
: "${OPENPDF_PDFIUM:?Set bundled runtime path}"
OPENPDF_TEST_BINARY=${OPENPDF_TEST_BINARY:-"$PWD/target/release/openpdf-app"}
OPENPDF_UX_DIR=$(mktemp -d)
OPENPDF_UX_PID=
cleanup() {
  if [ -n "$OPENPDF_UX_PID" ]; then kill "$OPENPDF_UX_PID" 2>/dev/null || true; fi
  rm -rf "$OPENPDF_UX_DIR"
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$OPENPDF_UX_DIR/config" "$OPENPDF_UX_DIR/data"
cp fixtures/forms.pdf "$OPENPDF_UX_DIR/test.pdf"
launch() {
  env -u WAYLAND_DISPLAY XDG_CONFIG_HOME="$OPENPDF_UX_DIR/config" XDG_DATA_HOME="$OPENPDF_UX_DIR/data" LIBGL_ALWAYS_SOFTWARE=1 "$OPENPDF_TEST_BINARY" "$OPENPDF_UX_DIR/test.pdf" > "$OPENPDF_UX_DIR/app.log" 2>&1 &
  OPENPDF_UX_PID=$!
  tries=0
  until xdotool search --name '^OpenPDF Desktop$' >/dev/null 2>&1; do
    tries=$((tries+1));if [ "$tries" -ge 30 ]; then cat "$OPENPDF_UX_DIR/app.log";exit 1;fi
    sleep 1
  done
  sleep 2
}
launch
python3 scripts/gui_signature_acceptance.py "$OPENPDF_UX_DIR/test.pdf"
python3 scripts/gui_close.py
wait "$OPENPDF_UX_PID"
OPENPDF_UX_PID=
launch
python3 scripts/gui_signature_acceptance.py "$OPENPDF_UX_DIR/test.pdf" --reopen
python3 scripts/gui_close.py
wait "$OPENPDF_UX_PID"
OPENPDF_UX_PID=
