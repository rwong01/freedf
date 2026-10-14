#!/bin/sh
# Optional distribution step. Requires an explicitly configured signing identity.
set -eu
: "${OPENPDF_CODESIGN_IDENTITY:?Set your Apple Developer signing identity}"
: "${OPENPDF_NOTARY_PROFILE:?Set the notarytool keychain profile}"
app="$1"
codesign --force --options runtime --timestamp --sign "$OPENPDF_CODESIGN_IDENTITY" "$app/Contents/MacOS/libpdfium.dylib"
codesign --force --options runtime --timestamp --sign "$OPENPDF_CODESIGN_IDENTITY" "$app"
ditto -c -k --keepParent "$app" "$app.zip"
xcrun notarytool submit "$app.zip" --keychain-profile "$OPENPDF_NOTARY_PROFILE" --wait
xcrun stapler staple "$app"
