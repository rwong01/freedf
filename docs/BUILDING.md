# Building and packaging

Rust 1.99.0, Python 3.12+ and a native C compiler are used. Cargo.lock pins Rust
packages; pdf_signer is retained as a documented source copy in third_party/pdf_signer, based on a pinned upstream commit with a parser security update. PDFium archive SHA-256 values are
in packaging/pdfium.lock.json. The GUI never downloads a runtime.

On Ubuntu 24.04 install libxkbcommon-dev, libxkbcommon-x11-0, libwayland-dev,
libgl1-mesa-dev, libegl1-mesa-dev and pkg-config. CUPS is optional for printing;
an unlocked Secret Service is required to create/reuse local signing identities.
Native file dialogs require a working xdg-desktop-portal backend.

```
python3 scripts/fetch_pdfium.py linux-x64
cargo build --release --locked
OPENPDF_PDFIUM="$PWD/vendor/pdfium/lib/libpdfium.so" target/release/openpdf-app fixtures/forms.pdf
```

On Windows use the MSVC toolchain with Visual Studio C++ Build Tools, run the
fetch script with win-x64, then cargo build --release --locked. Place pdfium.dll
beside openpdf-app.exe (or set OPENPDF_PDFIUM explicitly during development).
On macOS use Xcode command-line tools and the matching mac-x64/mac-arm64 archive.

To package, run scripts/third_party.py, then scripts/package.py PLATFORM --binary
PATH. The Linux script produces a portable tar.gz and an Ubuntu 24.04 .deb.
For Windows, compile packaging/windows/installer.nsi with NSIS after staging.
The installer registers Open With without replacing an existing default PDF app.
Uninstallation retains documents, signing identity, and profile data.

The available environment cross-built Windows with a locally extracted MinGW
compiler and x86_64-pc-windows-gnu Rust target. This proves compilation, not
Windows runtime behavior. Native MSVC builds are configured in CI but have not
been observed here. macOS builds have not been run.

## Headless GUI testing

Use a dedicated Xvfb server, Mesa llvmpipe, xdotool and scrot. For example:

```
Xvfb :106 -screen 0 1440x1000x24 -nolisten tcp &
env -u WAYLAND_DISPLAY DISPLAY=:106 LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe \
  OPENPDF_PDFIUM="$PWD/vendor/pdfium/lib/libpdfium.so" target/debug/openpdf-app fixtures/forms.pdf
DISPLAY=:106 python3 scripts/gui_smoke.py forms
```

Use disposable generated PDFs. gui_smoke.py explicitly refuses DISPLAY=:0.
Screenshots and file validation are still required after executing input scripts.
A headless server can exercise the actual UI; it does not require a physical GPU.
The implementation host lacked administrator installation. Tools were extracted
under tools/sysroot, and the local Xvfb test binary's keyboard tool directory was
adjusted to /tmp/bin. Neither adjustment is shipped in application artifacts.

For secure-store integration, run scripts/keychain_smoke.py under dbus-run-session
with gnome-keyring-daemon available. It creates a private temporary encrypted
keyring, runs two application processes, compares fingerprints, and cleans up.
Never substitute a mocked or plaintext store in the production application.

For automated GUI assertions, with system test tools installed:

```sh
OPENPDF_PDFIUM="$PWD/vendor/pdfium/lib/libpdfium.so" \
  xvfb-run -a -s '-screen 0 1440x1000x24' dbus-run-session -- scripts/gui_acceptance.sh
```

This asserts saved form text, actual clipboard contents and persisted native
rectangle annotations. Search screenshots are evidence for visual inspection;
this subset does not establish complete release acceptance.

GitHub runner labels were checked against the [official runner reference](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job).
The configured macOS labels select ARM64 and Intel separately.

For the complete interactive digital-signing acceptance test, start
`scripts/gui_signing_session.py` under a fresh `dbus-run-session` on Xvfb with
gnome-keyring and the GTK desktop portal installed. It opens two successive app
processes, uses a private temporary encrypted keyring, checks identity reuse and
independently verifies both files with Poppler. Follow its required evidence file
names in the Save dialog. `scripts/gui_close.py` sends a normal window-close event
when no window manager is present. Never run these harnesses on a user's session.
