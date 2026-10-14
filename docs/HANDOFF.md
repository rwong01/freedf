# Engineering handoff

OpenPDF Desktop is a working development application, **not v1 complete or
production hardened**. Source is in https://github.com/rwong01/freedf, branch main.
See FEATURE_MATRIX.md for every release gate and SECURITY.md before handling
untrusted documents.

## Implemented and exercised

Six Rust workspace crates separate the desktop UI, PDF document/edit model,
PDFium worker, persistence, local identity and signing adapter. Actual headless
GUI tests cover reading, thumbnails/navigation, search, selection/copy, multiple
tabs, form text/checkbox save, annotation creation/movement/resize/undo, and
visual signatures. Form/annotation structures and appearances persist as PDFs.

The full digital-signing GUI workflow was exercised with native GTK Save dialogs
and a real isolated encrypted Secret Service: first-use identity generation,
signing, reopening the output, application restart, identity reuse and a second
signature. Both outputs validate in Poppler; OpenSSL extraction confirms the same
embedded certificate in both documents and the local profile. The test identity
was removed after the run; no private key is included in the repository.

Twelve workspace tests and the separately enabled PDFium rendering test passed.
Linux and Windows-target Clippy checks passed with warnings denied. qpdf checked
serialized PDFs, Poppler validated two incremental signatures and rejected
signed-byte tampering. These checks do not establish certificate-chain trust.

Native GitHub Actions has also executed Windows unit tests, PDFium integration,
release compilation and real Credential Manager identity reuse. Linux and macOS
ARM64 build/test/package jobs have passed on earlier revisions. Latest source
revalidation is tracked at https://github.com/rwong01/freedf/actions. Interactive
Windows/macOS desktop behavior, printers, and clean installation remain unverified.

## Build and launch

See BUILDING.md for native dependencies. From the repository root:

```sh
python3 scripts/fetch_pdfium.py linux-x64
cargo build --release --locked
OPENPDF_PDFIUM="$PWD/vendor/pdfium/lib/libpdfium.so" target/release/openpdf-app fixtures/forms.pdf
```

Windows developers use the MSVC toolchain and `win-x64` runtime archive. End users
of the packaged application do not need Rust. Runtime PDFium and license notices
are included; the app never downloads dependencies or sends documents online.

Local build outputs are under `dist/` (ignored by Git):

- `windows-x64/openpdf.exe`: portable Windows executable with adjacent pdfium.dll.
- `OpenPDF-Desktop-0.1.0-windows-x64-setup.exe`: per-user NSIS installer.
- `OpenPDF-Desktop-0.1.0-windows-x64.zip`: portable Windows distribution.
- `linux-x64/openpdf`: Linux executable with adjacent libpdfium.so.
- `OpenPDF-Desktop-0.1.0-linux-x64.deb`: Ubuntu 24.04 x64 package.
- `OpenPDF-Desktop-0.1.0-linux-x64.tar.gz`: Linux archive.
- `OpenPDF-Desktop-0.1.0-source.tar.gz`: corresponding source with locked Rust
  dependency sources; it resolves offline from an empty Cargo home.

CI produces native platform artifacts. Older CI artifacts predate fixes discovered
in GUI tests; prefer artifacts from the latest source revision. No code signing,
notarization or production release certification is claimed.

## Remaining release blockers and continuation

1. Finish clean Windows installation/uninstallation, Explorer association, actual
   UI signing and native print tests; execute macOS UI/Keychain/printing tests.
2. Isolate untrusted parsing/rendering in a sandboxed process, review the pinned
   PDFium build, fuzz parser boundaries and resolve the RSA dependency advisory
   and unmaintained parser warning. `cargo audit` deliberately remains failing.
3. Complete links, named bookmarks, Unicode form/free-text appearance support,
   existing signature-field filling, signature cropping and identity lifecycle
   management. Validate the complete forms/annotation corpus and accessibility.
4. Validate Acrobat, Chrome, Edge and Preview separately. Poppler results do not
   imply these applications passed.
5. Measure real startup, document-wide search, GUI memory and large-document
   latency; add failure injection for disk-full, locked and permission-denied saves.

Keys are exportable software keys encrypted at rest, with a random password in the
OS store. Non-exportable native signing providers are not implemented. Private-key
transient copies in upstream code and in-process PDF parsing remain limitations.
The generated certificate is self-signed and does not independently prove identity.
Deleting annotations is not redaction or secure erasure.

Continuation commands: `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, the
PDFium ignored test, scripts/validate_external.py, scripts/gui_acceptance.sh and
scripts/gui_signing_session.py. Preserve signing identities and original signed
PDF bytes. Do not mark unexecuted workflows complete.
