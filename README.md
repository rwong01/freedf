# OpenPDF Desktop

An offline Rust desktop PDF reader, annotator, form filler, and local digital signer.
GPL-3.0-or-later. Repository: https://github.com/rwong01/freedf

**Development release, not v1 complete.** This repository includes working PDFium
rendering, native PDF annotation/form persistence, and real incremental ECDSA PDF
signatures. See [the feature matrix](docs/FEATURE_MATRIX.md) for incomplete and
unverified release requirements. Do not infer production readiness from a build.

## Build and run

Use Rust 1.99 or later. Runtime PDFium is deliberately bundled, never downloaded
by the application. See [BUILDING](docs/BUILDING.md) for dependencies and packaging.

```sh
python3 scripts/fetch_pdfium.py linux-x64
cargo build --locked
OPENPDF_PDFIUM="$PWD/vendor/pdfium/lib/libpdfium.so" cargo run --locked -p openpdf-app -- fixtures/forms.pdf
```

The application supports PDF file arguments, drag and drop, and native dialogs.
Protected/signed documents disable ordinary editing. Digital signing produces a
new copy. Local identities require an unlocked OS credential store; there is no
plaintext fallback. Local certificates are self-signed, not independently trusted.

## Validation

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
OPENPDF_PDFIUM="$PWD/vendor/pdfium/lib/libpdfium.so" cargo test -p openpdf-render -- --ignored
OPENPDF_PDFIUM="$PWD/vendor/pdfium/lib/libpdfium.so" cargo run -p openpdf-app -- --validate-fixtures fixtures
qpdf --check fixtures/signed-twice.pdf
pdfsig -nocert fixtures/signed-twice.pdf
```

No Acrobat, Edge, Chrome or Preview compatibility is implied by Poppler checks.
See [COMPATIBILITY](docs/COMPATIBILITY.md), [SECURITY](docs/SECURITY.md), and
[USER_GUIDE](docs/USER_GUIDE.md).
