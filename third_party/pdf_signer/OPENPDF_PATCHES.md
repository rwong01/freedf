# OpenPDF maintained source copy

Upstream: https://github.com/StrategicProjects/pdf_signer
Revision: 6cc0218100d9ffc038f8dccee3b707e5bd136100 (0.3.2).
License: GPL-3.0-or-later; authors retained in Cargo.toml and source.

Changes: upgraded lopdf from 0.36 to 0.44 to remove RUSTSEC-2026-0187
(unbounded parser recursion). Application tests and independent Poppler
signature checks must pass after any upstream or local update.

The RSA dependency remains flagged by RUSTSEC-2023-0071. OpenPDF generates
P-256 identities; RSA private-key operations are not offered by the app.
The library still supports them and must not be treated as audited.
