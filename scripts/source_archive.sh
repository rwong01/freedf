#!/bin/sh
# Create corresponding source, including exact dependency sources, for GPL distribution.
set -eu
mkdir -p dist/source/.cargo
cargo vendor --locked --offline dist/source/vendor > dist/source/.cargo/config.toml
# Rewrite the vendor source path relative to the extracted source root.
python3 - <<'PY'
from pathlib import Path
p=Path('dist/source/.cargo/config.toml')
s=p.read_text().replace('directory = "dist/source/vendor"','directory = "vendor"')
p.write_text(s)
PY
git archive HEAD | tar -x -C dist/source
tar -czf dist/OpenPDF-Desktop-0.1.0-source.tar.gz -C dist source
