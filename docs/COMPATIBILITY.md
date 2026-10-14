# Compatibility evidence

Executed on Ubuntu 24.04 / WSL, x86_64, 2026-10-14. No Acrobat, Chrome, Edge,
Apple Preview, clean Windows installation or macOS runtime was available.

| Check | Result | Evidence |
|---|---|---|
| PDFium opens/renders generated form fixture | Pass | evidence/gui-reader.png and ignored renderer integration test |
| Actual GUI text entry + checkbox + Save | Pass | evidence/gui-forms-saved.png, evidence/gui-filled.pdf |
| Poppler extracts GUI-entered text | Pass | pdftotext output includes `GUI Alice` |
| qpdf validates saved GUI form PDF | Pass | No syntax/stream encoding errors |
| Actual GUI rectangle creation + Save | Pass | evidence/gui-annotation.png; qpdf JSON includes /Subtype /Square and /AP |
| Search across three pages | Pass | evidence/gui-search_000.png shows 3 results and highlighted occurrence |
| Form + native annotation and typed signature fixture serialization | Pass | fixtures/edited.pdf; model roundtrip tests |
| Actual GUI select + copy | Pass | evidence/gui-copy.png; X clipboard returned selected sentence |
| Draw/place/save visual signature through GUI | Pass | evidence/gui-drawn-signature.pdf; independent Poppler rendering in poppler-drawn-signature.png |
| Reopen PDFs in two document tabs | Pass | evidence/gui-copy.png shows both reopened files |
| One ECDSA CMS signature | Pass | Internal verifier and Poppler pdfsig |
| Second incremental signature | Pass | Both signatures valid in Poppler; first does not cover appended revision |
| Signed-byte tampering | Pass | Poppler reports Digest Mismatch; internal verification false |
| OS-protected identity reused across two processes | Pass | evidence/keychain-test.txt; same fingerprint; evidence/keychain-signed.pdf |
| qpdf validates twice-signed PDF | Pass | No syntax/stream encoding errors |
| Independent signer trust | Not established | Self-signed certificate; pdfsig used -nocert |
| Pixel-level renderer comparison | Unverified | No visual-regression pass claimed |
| Third-party application form/annotation interaction | Unverified | A parser check does not establish interactive interoperability |
| Packaged Linux runtime loading | Pass | dist/linux-x64/openpdf generated and verified fixtures with its adjacent PDFium, without OPENPDF_PDFIUM |
| Native Windows/macOS launch and printing | Unverified | Cross-build does not establish runtime correctness |
| Acrobat/Chrome/Edge/Preview | Unverified | Not executed |

Poppler 24.02.0 and qpdf 11.9.0 were used from locally extracted Ubuntu packages.
Poppler emitted an NSS database initialization warning in this host; `-nocert`
intentionally skipped trust validation. Cryptographic signature validation still
reported valid, and the tampered file produced Digest Mismatch. This must not be
reported as a trusted certificate-chain result.

Initial debug-run measurement: first fixture render at 816px took 67 ms; complete
internal fixture generation/signing/validation took 2,890 ms. These are individual
observations, not performance guarantees. The later optimized release run measured 16 ms for that render, 308 ms for the
complete fixture workflow, and 34,492 KiB peak RSS (the fixture process, not GUI
memory). See fixtures/internal-validation.json and evidence/fixture-resource-use.txt.
