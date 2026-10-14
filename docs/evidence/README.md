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
  points using the actual inspector and saved. Movement acceptance remains pending.
- keychain-test.txt/keychain-signed.pdf: production Secret Service storage tested
  with an isolated encrypted keyring, reused in two separate application processes.

Earlier screenshots may show UI defects corrected later (notably duplicate saved
form text behind input controls). Prefer gui-ci-* images when available for the
final overlay appearance. Search evidence gui-search_000.png contains the query;
gui-search.png was captured before text entry and is not a search assertion.

Executing an input script alone is not proof of a workflow. Where assertions exist,
they check the saved PDF structure, extracted text, actual clipboard or signatures.
No screenshots here establish Windows/macOS/Acrobat/Chrome/Edge/Preview acceptance.
