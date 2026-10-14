# Execution evidence

Synthetic documents and screenshots from actual local runs on Ubuntu 24.04 WSL.
These files contain only generated test names/certificates, never private keys.

- unit-tests.txt: workspace test results (renderer integration separately).
- renderer-tests.txt: actual PDFium rendering, text search and rotation checks.
- clippy.txt: workspace/all-targets lint results with warnings denied.
- cargo-audit.json: audit **fails** on RSA advisory; see SECURITY.md. No suppression.
- windows-cross-build.txt: Linux GNU cross-compilation, not Windows execution.
- linux-release-build.txt: Linux release build.
- gui-reader/form/search/copy/annotation images: actual eframe window driven by
  xdotool on an isolated Xvfb display, with Mesa software rendering.
- gui-drawn-signature.pdf and poppler-drawn-signature.png: mouse-drawn signature
  placed and saved in the GUI, then rendered by independent Poppler.
- gui-resized-signature.pdf: the signature width was changed from 270 to 180 PDF
  points using the actual inspector and saved. Canvas movement and undo were subsequently verified in gui-movement-validation.json, following page annotation references rather than orphaned objects.
- keychain-test.txt/keychain-signed.pdf: production Secret Service storage tested
  with an isolated encrypted keyring, reused in two separate application processes.

Earlier screenshots may show UI defects corrected later (notably duplicate saved
form text behind input controls). Prefer gui-ci-* images when available for the
final overlay appearance. Search evidence gui-search_000.png contains the query;
gui-search.png was captured before text entry and is not a search assertion.

Executing an input script alone is not proof of a workflow. Where assertions exist,
they check the saved PDF structure, extracted text, actual clipboard or signatures.
No screenshots here establish Windows/macOS/Acrobat/Chrome/Edge/Preview acceptance.

The full cryptographic GUI workflow used a fresh isolated Secret Service and
real GTK portal dialogs. First signing exposed a zero/one-based page mismatch;
it was corrected at the signing adapter boundary and is covered by a regression
test asserting the widget belongs to the intended first/last page. The corrected
debug GUI created an identity, signed, closed, restarted, reused the identity and
signed again. Both outputs pass Poppler. OpenSSL extraction independently proves
the two embedded certificate hashes match each other and the local profile.
The second output was saved under the native dialog's default edited-signed.pdf
name, then renamed gui-cryptographic-second.pdf for evidence collection.
The temporary test profile and credential store were removed after the test.
