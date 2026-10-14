# Release-one feature matrix

Status as of 2026-10-14. **v1 is NOT complete.** “Tested” applies only to the
specific evidence given, not to all platforms or all input PDFs. “Implemented”
means code exists but the complete acceptance workflow remains unverified.

| Requirement | Status | Evidence / remaining work |
|---|---|---|
| Modular Rust workspace and GPL license | Tested | Workspace builds; source split into app/document/render/persistence/identity/signatures |
| Windows x64 executable | Tested | GNU cross-build on Linux; not run on Windows |
| Windows clean-machine installation/uninstallation | In progress | NSIS installer compiled locally; clean-machine acceptance requires Windows VM |
| macOS Apple Silicon/x64 | In progress | Runtime checksums, bundle script and CI matrix; no macOS execution |
| Linux desktop launch | Tested | Real eframe window under Xvfb + Mesa CPU renderer; screenshots in evidence |
| Open via command-line argument | Tested | GUI opened generated forms.pdf |
| Native file dialog and drag/drop | Implemented | Code present; dialog/drag-drop end-to-end tests pending |
| OS file associations/Open With | Implemented | NSIS per-user ProgID/OpenWith registration; Linux desktop entry; macOS metadata; not installation-tested |
| Multiple document tabs | Tested | Two PDFs reopened in the actual window (gui-copy.png); eight-tab limit |
| Recent files/window/preferences persistence | Implemented | eframe persistence; reopen workflow not fully exercised |
| PDFium rendering | Tested | Actual page rendering, text extraction, original forms fixture |
| Lazy rendering/background owner worker | Tested | GUI screenshots/status; no startup rasterization of every page |
| Continuous scrolling/single-page view | Implemented | Virtual page layout; manual full navigation test pending |
| Thumbnails | Tested | Three actual PDF thumbnails in headless GUI |
| Zoom/fit/rotation | Implemented | Coordinate unit tests pass; all UI combinations unverified |
| Crop/rotation/DPI transforms | Tested | Coordinate round trips; independent visual alignment cases still needed |
| Mixed sizes/landscape/scanned/Unicode/embedded fonts | In progress | PDFium supports these; project-specific corpus coverage incomplete |
| Hundreds of pages | Implemented | 500-page generated fixture; no performance acceptance claim |
| Password-protected reading | Implemented | Password retry UI, readonly editing policy; fixture testing pending |
| Malformed input | Tested | Parser rejects invalid fixture; fuzzing and isolation still required |
| Bookmarks | Implemented | Direct page destinations; named/complex destinations incomplete |
| Internal links | Not started | No enabled link control claims support |
| External links with confirmation | Not started | No document URL/action is executed automatically |
| Document-wide search/results/highlights | Tested | GUI found three instances across three pages; screenshot evidence |
| Selection and clipboard | Tested | Actual drag selection and Ctrl+C; isolated X clipboard returned selected sentence (gui-copy.png) |
| Single/multiline text fields | Tested | ASCII form save/reopen unit tests; single-line GUI entry + Poppler extraction |
| Checkbox | Tested | GUI checked and saved; field value and appearance persisted |
| Radio groups | Tested | Shared parent field value + widget AS/AP roundtrip in unit test |
| Combo/list fields | Tested | Value/export options and arrays supported; combo roundtrip unit test; complete GUI coverage pending |
| Readonly form fields | Tested | Disabled UI and serializer refusal |
| Existing signature field filling | Not started | Detected and labeled; signer currently appends a new field |
| XFA detection | Implemented | Editing blocked; no XFA engine bundled |
| Unicode field/free-text appearances | Not started | ASCII-only writes enforced; no silent glyph substitution |
| Native highlight/underline/strikethrough | Implemented | Standard annotation dictionaries and AP; full GUI text-markup workflow pending |
| Ink, note, free text, rectangle, ellipse, line | Tested | Fixture creation/serialization; rectangle also drawn and saved through actual GUI |
| Comment text/color/opacity/stroke | Implemented | Edit controls and PDF AP; broader tests pending |
| Move/resize/delete annotations | Implemented | Select + drag, inspector dimensions, Delete key; actual GUI acceptance pending |
| Annotation undo/redo | Tested | Model history roundtrip; GUI buttons wired |
| Existing third-party annotation editing | Not started | Existing objects preserved; only OpenPDF-owned annotations editable in current UI |
| Draw visual signature | Tested | Actual mouse drawing, placement, Save; Poppler independently renders native ink in gui-drawn-signature.pdf; resize workflow pending |
| Type visual signature | Tested | Two OFL fonts; transparent image generation, PDF persistence and independent Poppler rendering |
| Import signature image/transparency | Implemented | PNG/JPEG limits, alpha mask XObject; end-to-end pending |
| Signature cropping/background cleanup | Not started | Accurately labeled in UI |
| Signature placement/movement/resizing | In progress | GUI drawing/placement/save and inspector resize (270→180 points) tested; movement acceptance pending |
| Saved reusable appearances/delete | Implemented | Explicit local save, private filesystem permissions; no default/rename UI yet |
| Initials/date/checkmark shortcuts | In progress | Initials via type; generic text/ink work; date/checkmark shortcuts pending |
| Flatten Copy | Not started | No misleading flatten button |
| First-use local identity setup | Tested | Actual UI name entry, native Save dialog, real protected identity, signed copy reopened; gui-cryptographic-first.pdf |
| Secure P-256 identity generation | Tested | Random identities differ; PKCS#12 wrong-password rejection |
| OS-protected identity persistence/reuse | Tested | CLI and two actual GUI processes; Poppler/OpenSSL confirm identical embedded certificate fingerprints matching the protected profile |
| Windows Credential Manager/macOS Keychain | Implemented | Native keyring backends compiled/configured; runtime unverified |
| Non-exportable CNG/Secure Enclave keys | Not started | Exportable key limitation disclosed; PKCS#12 signer needs external-provider adaptation |
| Certificate details/fingerprint/dates | Implemented | Preferences shows local certificate metadata |
| Identity import/export/renewal/rollover | Not started | Existing identity never silently replaced; migration not promised |
| Certificate-backed signature | Tested | ECDSA/SHA-256, CMS/CAdES detached, ByteRange; internal and Poppler validation |
| Visible digital signature | Tested | Signed fixture contains signature AP; wider visual interoperability unverified |
| PAdES B-B profile | In progress | Signer emits CAdES profile; independent conformance certification not performed |
| PAdES B-T/B-LT/B-LTA | Not started | No TSA/revocation networking offered in the app |
| Incremental signing/preservation | Tested | Two signatures both valid in Poppler; original byte prefix retained |
| Tampering detection | Tested | Internal verifier rejects; Poppler reports Digest Mismatch |
| Integrity/trust separation | Tested | UI wording separates integrity from self-signed identity; no trust root configured |
| Chain trust/revocation/timestamps | In progress | Details explicitly unknown/not checked; no online trust validation |
| DocMDP/FieldMDP protection | Tested | Conservative block implemented; allowed restricted incremental editing not supported |
| Save/Save As/dirty state | Tested | Save through GUI plus parse/reopen tests; native Save As dialog acceptance pending |
| Atomic replacement | Tested | Sibling temp/sync/replace; existing file replacement and invalid-output preservation tests |
| External modification detection | Implemented | Expected bytes checked before replacing current file; race window documented |
| Disk-full/locked-file/permission fault injection | Not started | Errors propagate; no exhaustive fault-injection evidence |
| Unsaved-close warning | Implemented | UI close guard; full automated acceptance pending |
| Windows printing | Implemented | Native PrintDlg/GDI path compiled; printer, ranges, copies via OS dialog; not run |
| Linux printing | Implemented | CUPS lp snapshot/fit/range/copies; disabled if spooler unavailable; no printer tested |
| macOS printing | Not started | Print disabled |
| JS/embedded execution/network privacy | Implemented | No action execution APIs; PDFium no V8/XFA; no runtime downloads or telemetry |
| Parser/renderer process sandbox | Not started | Native PDF parsing is in process; security release blocker |
| File/page/bitmap/cache limits | Implemented | 256 MiB files, 10k pages, 4096 render dimension, 128 MiB texture cache |
| Cancellation/obsolete request discard | In progress | Bounded queue; closed tabs discarded; zoom requests may finish unnecessarily |
| Startup/render/search/save/memory benchmarks | In progress | Optimized fixture run: 816px render 16ms, full fixture workflow 308ms, peak RSS 34,492KiB; GUI/startup/search benchmarks remain |
| Accessibility/keyboard/focus | In progress | AccessKit, text controls and shortcuts present; assistive-technology acceptance pending |
| Dependency audits | Blocked | Audit executed; vulnerable lopdf updated. RSA timing advisory and unmaintained ttf-parser warning remain; see SECURITY.md |
| License notices/source distribution | In progress | 597-package inventory, bundled native/font licenses, source archive with vendored dependencies; legal audit still pending |
| GUI automation | Tested | Xvfb actual-window form, search and annotation interactions; partial acceptance only |
| CI Windows/macOS/Linux | In progress | Native CI executing. Windows-only lint failure fixed; audit intentionally fails on remaining advisory. See CI run links |
| Acrobat/Chrome/Edge/Preview compatibility | Not started | Not available/tested; Poppler results do not establish these |
| Full reader/forms/signing/release acceptance | In progress | Individual evidence above; all requested v1 workflows not yet satisfied |
