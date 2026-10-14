# Using this development build

Open a PDF with Open, drag it onto the window, or launch the executable with a PDF
path. Each document uses a tab. Thumbnails navigate pages; the toolbar offers
zoom, fit, rotation, and a single-page mode. Search scans text layers; scanned
images without a text layer have no searchable text.

Select and drag across text; Ctrl+C copies the selected characters. Highlight,
Underline and Strike tools use selected character rectangles. Ink draws on the
page. Rectangle, ellipse and line tools drag out shapes. Comments and text boxes
use the text configured in the Comments sidebar. Select an OpenPDF annotation to
move it by dragging; edit dimensions, color and opacity in Comments. Delete
removes it. Ctrl+Z/Ctrl+Y undo and redo model changes.

Click editable form fields on an unrotated page, or use Forms in the sidebar.
Readonly fields are disabled. ASCII text appearances are supported; unsupported
characters produce a save error rather than disappearing. Existing form objects
and unrelated annotations are preserved. XFA, encrypted and signed documents are
readonly in the ordinary editor. View rotation does not change saved page rotation.

Fill & Sign creates a visual signature by drawing, typing with a bundled
handwritten font, or importing PNG/JPEG. Place it, then select it to move/resize.
Explicitly save an appearance if you want to reuse it. Typed initials use the same
workflow. Image cropping and background removal are not implemented.

Digitally sign creates a separate signed copy. On first use, enter a display name
and optional email. The app generates and protects a local identity with your OS
credential service and reuses it on later runs. If secure storage is unavailable,
it stops instead of saving an unprotected key. Select an annotation/signature
first to use its bounds; otherwise the app uses a visible box near the page bottom.
Every operation requires Sign & save confirmation. Certificate identity is
self-signed: integrity verification is not independent verification of a person.

Save writes only after serialization succeeds. Save As chooses another path.
An asterisk on the tab indicates unsaved edits; errors leave the document dirty.
Close warnings protect unsaved changes. Signed copies are opened in a protected
tab so ordinary editing cannot silently invalidate them.

Windows printing uses the native printer dialog. Linux printing requires CUPS
and uses a committed PDF snapshot; the current UI uses the configured printer's
paper/orientation and allows range/copies/fit. Actual printer output remains
unverified. macOS printing is disabled until implemented.

This is not a replacement-ready v1 release. Consult FEATURE_MATRIX.md and
COMPATIBILITY.md before depending on a workflow.
