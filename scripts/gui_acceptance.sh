#!/bin/sh
# Run inside xvfb-run on a dedicated display, with a built application and PDFium.
set -eu
: "${DISPLAY:?Start this script with xvfb-run}"
: "${OPENPDF_PDFIUM:?Set the absolute PDFium library path}"
OPENPDF_TEST_BINARY=${OPENPDF_TEST_BINARY:-"$PWD/target/release/openpdf-app"}
OPENPDF_TEST_DIR=$(mktemp -d)
OPENPDF_TEST_PID=
cleanup() {
  if [ -n "$OPENPDF_TEST_PID" ]; then kill "$OPENPDF_TEST_PID" 2>/dev/null || true; fi
  rm -rf "$OPENPDF_TEST_DIR"
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$OPENPDF_TEST_DIR/config" "$OPENPDF_TEST_DIR/data" docs/evidence
cp fixtures/forms.pdf "$OPENPDF_TEST_DIR/gui.pdf"
env -u WAYLAND_DISPLAY XDG_CONFIG_HOME="$OPENPDF_TEST_DIR/config" XDG_DATA_HOME="$OPENPDF_TEST_DIR/data" LIBGL_ALWAYS_SOFTWARE=1 "$OPENPDF_TEST_BINARY" "$OPENPDF_TEST_DIR/gui.pdf" > "$OPENPDF_TEST_DIR/app.log" 2>&1 &
OPENPDF_TEST_PID=$!
tries=0
until xdotool search --name '^OpenPDF Desktop$' >/dev/null 2>&1; do
  tries=$((tries+1))
  if [ "$tries" -ge 30 ]; then cat "$OPENPDF_TEST_DIR/app.log"; exit 1; fi
  sleep 1
done
sleep 2
python3 scripts/gui_smoke.py forms --output docs/evidence/gui-ci-forms.png
pdftotext "$OPENPDF_TEST_DIR/gui.pdf" "$OPENPDF_TEST_DIR/form.txt"
grep -q 'GUI Alice' "$OPENPDF_TEST_DIR/form.txt"
python3 scripts/gui_smoke.py copy --output docs/evidence/gui-ci-copy.png
xclip -o -selection clipboard > docs/evidence/gui-clipboard.txt
grep -q 'Select this sentence' docs/evidence/gui-clipboard.txt
python3 scripts/gui_smoke.py search --output docs/evidence/gui-ci-search.png
python3 scripts/gui_smoke.py annotation --output docs/evidence/gui-ci-annotation.png
qpdf --check "$OPENPDF_TEST_DIR/gui.pdf"
qpdf --json "$OPENPDF_TEST_DIR/gui.pdf" > "$OPENPDF_TEST_DIR/pdf.json"
python3 - "$OPENPDF_TEST_DIR/pdf.json" <<'PY'
import json,sys
j=json.load(open(sys.argv[1]))
assert '/Square' in json.dumps(j), 'The GUI-created rectangle was not persisted'
PY
cp "$OPENPDF_TEST_DIR/gui.pdf" docs/evidence/gui-ci-saved.pdf
printf '%s\n' 'GUI assertions passed: saved text value, real clipboard text, native rectangle persistence.'
