# Signing UX update

Application source: `0999bc3e6d8c14e26d3f550acacac18d128d8529`.
Native build: https://github.com/rwong01/freedf/actions/runs/38676328436

The toolbar now separates **Add signature** (handwriting) from **Sign with
certificate** (tamper-evident signed copy). A single placement action and an
optional **Remember for next time on this device** checkbox replace competing
placement/save-appearance buttons. Saved signatures have previews and readable
names. Certificate details are secondary to plain-language instructions.

Placement chooses a proportionate starting size, fits inside a dragged box,
and visually fits an editable signature widget when present. Transparent image
padding and unused drawing space are trimmed; imported proportions are preserved.
Selected signatures have corner handles plus Smaller/Larger/Auto size/Delete/Done
controls directly in the toolbar. Movement and sizing are constrained to the page.
Completed gestures use one undo step. Certificate signing preserves existing
handwriting instead of covering it with a second stamp.

Less frequent annotation tools are grouped under More tools. Save feedback remains
visible through background renders; the status bar shows unsaved changes instead
of developer cache statistics.

## Package

`dist/OpenPDF-Desktop-0.1.0-windows-x64-signature-ux.zip`

This is the native MSVC Windows CI build, not a GNU cross-build. Extract the whole
archive into a new folder and run `windows-x64/openpdf.exe` with its adjacent
PDFium DLL. Existing profile and saved identities remain in the normal OS profile.
The previous investigation ZIP is preserved. Package hashes are recorded in
`evidence/signature-ux/windows-build.json` and `dist/signature-ux-build.json`.

## Validation

- 16 workspace tests, formatting, Linux and Windows-target Clippy passed locally.
- Real PDFium rendering integration passed.
- Debug and optimized Linux GUI signing regressions passed: typed/drawn placement,
  remembered appearance reuse, corner resizing, undo/redo, automatic sizing,
  drag-to-fit, PDF persistence, and a second-process reopen.
- Existing actual GUI form, clipboard, search, and annotation regression passed.
- qpdf checked saved structures and geometry. Poppler verified the certificate
  signature; PDFium and Poppler both confirmed unchanged handwriting-page pixels
  before and after certificate signing without an additional stamp.
- Native Windows, Linux, Apple Silicon macOS, and Intel macOS CI build jobs passed.
  This does not establish interactive Windows/macOS GUI acceptance.
- Defender custom scans returned no detection for the exact new ZIP, EXE, and
  PDFium DLL in run 38676797656. Hashes were matched to the downloaded package.
  Cloud checks were disabled on that hosted runner; this is limited scan evidence,
  not a malware-free guarantee. See evidence/signature-ux/defender/.
- The separate dependency audit still fails on the previously documented RSA
  advisory. The original cross-build investigation remains unresolved.

Manual cropping, opaque-background removal, existing cryptographic field filling,
full accessibility validation, and broader UI polish remain incomplete. Automatic
sizing does not recognize arbitrary printed signature lines: drag a target box
for those. This is a focused development update, not v1 completion.
