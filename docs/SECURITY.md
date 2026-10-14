# Security and privacy

This is a development release. **Do not treat it as hardened for hostile PDFs.**

PDFium and lopdf parse untrusted data inside the application process. A dedicated
thread is an ownership and responsiveness boundary, not a security sandbox.
Process isolation, per-process resource limits, fuzzing, and native dependency
security maintenance are release blockers. Input length/page/bitmap/cache limits
reduce accidental exhaustion but do not stop decompression bombs inside parsers.
The pinned PDFium build requires a current vulnerability review before release.

No telemetry, accounts, document uploads, runtime downloads, or PDF JavaScript
execution are implemented. The bundled PDFium build has no V8/XFA support. The app
does not execute document actions, attachments, or external URLs. Signature
verification uses an empty trust store and does not fetch revocation information.
It reports mathematical integrity separately from identity trust. Timestamps and
revocation must never be inferred from local signing time.

Local signing identities use CSPRNG-generated ECDSA P-256 keys and SHA-256,
self-signed X.509 certificates, and encrypted PKCS#12. A random 256-bit password
is stored using the OS credential service. Windows uses Credential Manager,
macOS Keychain, Linux Secret Service. Failure to access secure storage is fatal
to identity creation/signing; there is no plaintext file fallback. The encrypted
identity file and public metadata live in the application profile.

Keys are exportable software keys, **not CNG non-exportable keys, TPM keys, or
Secure Enclave keys**. The current signer requires complete in-memory PKCS#12.
Top-level secrets are zeroized; upstream parser/signing allocations may contain
copies until freed. Swap/core dumps and compromise of the logged-in account are
outside the current protection guarantee. Linux tests cover an encrypted Secret
Service keyring, not every desktop's service configuration. Windows/macOS stores
are not runtime-tested. Certificates do not independently establish legal identity.

Identity import, renewal, backup, replacement and migration are not yet exposed.
Keep the profile and OS credential store intact. Copying identity.json alone is
not a usable backup. Never delete it to “fix” a damaged identity; inspect/recover
it first. Uninstallation deliberately preserves identities.

Signature appearances are sensitive assets. They are saved only on explicit
request, with private Unix directory/file permissions; platform profile ACLs
apply on Windows. They are not encrypted by the identity vault. Avoid shared
profiles and untrusted processes under the same account.

Saves serialize/validate before replacing a destination through a private sibling
temporary file. Unix syncs both file and directory. Windows file replacement uses
tempfile's platform implementation; antivirus/locking/filesystem differences
require real Windows fault tests. Expected original bytes detect external edits,
but a race remains between checking and replacing a file. There is no crash
recovery journal. Native printer/spooler behavior and crash-time temporary-file
cleanup need additional validation.

Existing signed/certified PDFs cannot be ordinarily edited. Signing appends bytes
and verifies every recognized signature before writing a new copy. Documents with
DocMDP, FieldMDP or field locks are conservatively refused. General permitted
incremental editing and existing signature-field filling are not supported.

The signer is a small upstream GPL project and has not received an independent
security audit here. Internal verification plus Poppler tests are useful evidence,
not formal PAdES conformance certification. Dependency audit results and remaining
findings must be resolved or explicitly assessed before any production release.

Local cargo-audit on 2026-10-14 found the upstream signer's lopdf 0.36 vulnerable
to RUSTSEC-2026-0187. The maintained source copy now uses lopdf 0.44; this finding
is removed from the resolved graph. RUSTSEC-2023-0071 remains for rsa 0.9.10
(no patched release reported by the audit database; [upstream advisory](https://rustsec.org/advisories/RUSTSEC-2023-0071.html)). Local keys/signatures use
ECDSA P-256, and the app does not expose RSA identity import or RSA private-key
operations. RSA verification and upstream optional RSA signing code remain in
the dependency. The audit fails deliberately; no advisory is ignored. Replacing
or separating that backend and reviewing exposure is a production-release gate.
The transitive ttf-parser 0.25.1 also has an unmaintained warning
(RUSTSEC-2026-0192). See evidence/cargo-audit.json for the exact report.

Ordinary annotation deletion is not secure erasure. Rewritten PDFs may retain
unreferenced prior objects and appearances; do not use this application for
redaction or assume deleting a visible signature removes all recoverable asset
bytes. Object pruning and a purpose-built redaction workflow remain future work.
