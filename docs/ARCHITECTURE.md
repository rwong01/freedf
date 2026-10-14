# OpenPDF Desktop architecture

Status: implementation in progress; not a v1 release. Decisions checked against
upstream source and current crate metadata on 2026-10-14.

## Components and trust boundaries

The Rust workspace separates the egui application, document editing model,
PDFium renderer/text adapter, persistence, local identity, and PDF signature
adapter. UI coordinates are transformed to PDF points through one page transform.
Rasterization never becomes the document editing model. Edits are explicit commands
with undo/redo; saves serialize from original bytes plus the current edit set.

PDFium owns rendering and text extraction. A dedicated worker owns all PDFium
objects; it returns owned bitmaps and text geometry. Visible pages are requested
lazily and bounded caches hold images. Generation IDs discard obsolete results.
Parsing is not yet process-sandboxed; this remains a release security limitation.
The bundled PDFium build excludes V8/XFA. No document action or JavaScript is invoked.

lopdf owns structural inspection and annotation/form serialization. Unsigned
documents may be rewritten. Signed documents are protected against ordinary edits;
incremental signing is a separate path. Conservatively reject documents with
DocMDP/FieldMDP policies until permission-aware editing is validated. Mere retention
of signed bytes does not prove that a later modification was authorized.

Persistence writes a private sibling temporary file, validates the produced PDF,
syncs it, and atomically replaces the destination. Dirty state clears only after
success. Signing operates on byte buffers and uses the same persistence layer.

Identity and signature appearance are independent. An exportable P-256 key and
self-signed certificate are wrapped in encrypted PKCS#12; its random password is
stored by Windows Credential Manager, macOS Keychain, or Linux Secret Service.
The encrypted identity is stored in the local application profile. Unavailable
OS credential storage is an error, never a plaintext fallback. This initial adapter
does not provide hardware/non-exportable keys. Zeroization is best effort; upstream
parsers may retain copies until deallocation. External signing providers require a
future upstream adapter because pdf_signer currently accepts complete PKCS#12 bytes.

## Evaluated implementations

* [pdfium-render 0.9.4](https://github.com/ajrcarey/pdfium-render): MIT/Apache-2.0;
  rendering, character geometry, search, annotations, form access and bookmarks.
  Uses dynamically bundled PDFium with explicit API feature 7881. Include all
  licenses from the [binary distribution](https://github.com/bblanchon/pdfium-binaries).
* [lopdf 0.44](https://github.com/J-F-Liu/lopdf): MIT; object-level modification,
  parser and incremental support. It is not a renderer or automatic appearance engine.
* [printpdf](https://github.com/fschutt/printpdf): MIT; useful for new documents,
  unnecessary for editing existing documents in this architecture.
* [eframe 0.36.2](https://github.com/emilk/egui): MIT/Apache-2.0; custom PDF canvases,
  accessibility integration, native windows, persistence. Selected over
  [iced](https://github.com/iced-rs/iced) (MIT) for direct canvas interaction.
  [Slint](https://slint.dev/license) has GPL/commercial/royalty-free options with
  additional terms; no material advantage here justifies changing the default.
* [pdf_signer](https://github.com/StrategicProjects/pdf_signer), commit
  `6cc0218100d9ffc038f8dccee3b707e5bd136100`, GPL-3.0-or-later: incremental CMS,
  ECDSA and verification. Small/young project, not an audited security boundary.
  Only offline B-B options are exposed. Independent pdfsig/qpdf tests are required.
  Its license requires GPL-compatible distribution, so this application is GPL-3.0-or-later.
* RustCrypto/P-256, rcgen and PKCS#12 provide established primitives and certificate
  encoding. No custom cryptographic primitive. Keyring 3.6.3 selected for its
  explicit native backend feature configuration; newer 4.x needs migration review.
* ring is available via rcgen; rustls is not needed for offline functionality.
  Native CNG/Keychain non-exportable signing is deferred until an external signer
  API can be audited and tested. No signing-time network access is enabled.

## Release gates

All requested workflows are tracked in FEATURE_MATRIX.md. A successful build alone
does not satisfy any GUI acceptance test. Windows is the primary intended platform;
Linux is the available implementation host. CI configuration is not evidence that
Windows or macOS has been run. No Acrobat or Apple Preview interoperability claim
can be made from this host.

OCR, page organization, conversion, redaction, and external identity providers can
be added behind document/renderer/identity interfaces. No dynamic plugin runtime
is introduced. True redaction must never reuse a rectangle annotation operation.

The signer is now a GPL source copy in `third_party/pdf_signer`, with upstream
authorship and exact base revision recorded. Its lopdf dependency was upgraded
to 0.44 after cargo-audit found the old parser's recursion vulnerability.
See `OPENPDF_PATCHES.md` there and SECURITY.md for the remaining RSA advisory.
