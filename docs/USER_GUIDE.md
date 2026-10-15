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

**Add signature** puts your handwriting on the PDF. Draw, type your name or
initials, import a PNG/JPEG, or choose a preview in Saved signatures. The optional
**Remember for next time on this device** checkbox keeps a reusable copy of the
handwriting; it does not save the PDF or add a certificate.

Choose **Place on document**, then click for an automatic starting size. To fit a
specific space, drag a box: the signature fits inside without stretching. Existing
editable signature boxes also provide visual fitting targets; this does not fill
their cryptographic signature field. Transparent image padding and unused drawing
space are trimmed automatically. Opaque backgrounds are preserved.

After placement, drag the signature to move it or drag a corner handle to resize.
**Smaller**, **Larger**, **Auto size**, **Delete**, and **Done** appear directly in
the toolbar. Proportions stay intact and placements stay within the page. Esc
cancels placement; Ctrl+Z/Ctrl+Y undo/redo a completed resize or placement. Choose
**Save** to keep it in the PDF. No certificate is required for this workflow.

**Sign with certificate** creates a separate signed copy that lets readers check
for changes to signed bytes. Finish editing first. On first use, enter a display name
and optional email. The app generates and protects a local identity with your OS
credential service and reuses it on later runs. If secure storage is unavailable,
it stops instead of saving an unprotected key. Existing placed handwriting stays
visible, with no additional certificate stamp covering it. If there is no placed
handwriting, a certificate stamp appears near the current page's bottom.
Every operation requires **Sign and save a copy** confirmation. Certificate identity is
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
