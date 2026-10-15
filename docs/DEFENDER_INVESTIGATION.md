# Windows Defender investigation

The user reported `Trojan:Win32/Bearfoos.A!ml` and quarantine of the Windows
application executable. Treat the distributed Windows development build as
under investigation. Do not instruct users to disable antivirus, add exclusions,
or restore the executable to run it. This is not merely a SmartScreen publisher
reputation warning. A false positive has not been established.

Local evidence at source revision `210f1311e50dab3349e62b94037c49c01b348b61`:

| Artifact | SHA-256 |
| --- | --- |
| Distributed portable ZIP | `91489e1e0ba0bf87016d47870e3043bd7c691f297c53b1ab710eff0f29edfe8d` |
| Distributed `openpdf.exe` (GNU cross-build) | `79e31a1fac8ee0adfe49a6dca3b4b91eb95e2013aa786b4da04fb22fbba14ad8` |
| Bundled `pdfium.dll` | `79d4676b656cfb1abcea88f9ade3b4b0826c5200382db5f4ec72a636c598c118` |

The packaged EXE matches the local compiler output; the packaged DLL matches the
downloaded runtime. These facts do not establish the absence of malware. The
user's quarantined file has not yet been independently hashed. Native Windows
build/functional tests in run `38652821545` passed, but that run did not record
Defender scans.

The manually dispatched `Windows Defender investigation` workflow scans the
distributed ZIP and executable, a previous independent native Windows CI build,
and optionally a fresh native build without restored Cargo/build caches. It
records hashes, signature status, engine/definition versions, protection state,
raw scanner output, and detection history. It uploads reports only and does not
execute the samples. Missing files, inactive protection, hash mismatches, and
scan errors fail the investigation; they never count as clean results.

The scanner uses Microsoft's `-DisableRemediation` custom-scan option to ignore
file exclusions and print findings without requesting remediation. This does
not turn off real-time protection or restore quarantined files. Real-time
quarantine can still make a sample unavailable, which fails the check. The
workflow does not change exclusions, cloud protection, or allowlists.

Initial run `38664357117` stopped before downloading samples: the hosted Windows
image reported active antivirus but disabled real-time protection. This is an
environment failure, not a malware finding or a clean scan. The preparation step
now explicitly enables real-time protection and verifies it before proceeding.
The second run exposed a scanner-version parsing bug (Windows includes a build
hash in its human-readable version) and a transient Defender update RPC failure.
The script now sorts numeric version components and retries updates up to three
times. Neither failed attempt is a clean scan or a malware finding.

A no-detection result is specific to that artifact and scanner configuration.
Windows Server CI can differ from consumer Windows cloud/behavioral detection.
Different toolchains produce different binaries; a native build passing while
the cross-build fails is a diagnostic clue, not a reason to relabel, obfuscate,
or repeatedly modify bytes to bypass detection. Investigate source/dependency
behavior and compare findings before recommending a replacement build.

If the hosted runner lacks active Defender, use an isolated disposable Windows
VM with current Defender and no personal documents or signing identities. Keep
protection active. First scan without executing the sample. Any subsequent
runtime investigation needs separate isolation and explicit observation of
processes, network activity, and persistence. Continue ordinary source work and
Linux GUI tests while the Windows artifact remains under investigation.

Microsoft submission is a parallel escalation, not a prerequisite to gathering
local evidence. No submission or malware-free certification has been made.

## First successful scans

Run `38664666826` scanned the previous native MSVC CI package successfully using
Defender engine `1.1.26090.9`, platform `4.18.26090.9`, definitions `1.461.31.0`:

| File | SHA-256 | Result |
| --- | --- | --- |
| Native CI EXE | `91a054844df3d2045bd7808182288c27aac36916b38ec8d5110ba3d87023056e` | No detection in custom scan |
| Native CI ZIP | `32739f613df75bbb6690382a0da32e265a5a196d9e751c7155ef34c41f54ab25` | No detection in custom scan |
| PDFium DLL | `79d4676b656cfb1abcea88f9ade3b4b0826c5200382db5f4ec72a636c598c118` | No detection in custom scan |

The runner reported real-time protection active, but MAPS/cloud reporting off,
sample submission disabled, and default whole-drive exclusions. The custom scan
option ignores file exclusions; this result does **not** demonstrate unrestricted
real-time or cloud/behavioral detection. No sample was executed by this workflow.
It does not clear the differently compiled distributed EXE or the user's PC.

That run could not fetch the distributed ZIP because release lookup did not
expose the draft to the read-only workflow token. Automatic approval review
rejected a proposed repository-write token for downloading the draft. That
change was not pushed or run. The workflow instead tries the asset-by-ID API,
which GitHub documents as requiring only contents:read, and verifies the pinned
hash. If GitHub also denies this request, the original scan remains blocked;
no elevated token or publication is used. All jobs retain read-only permissions.

Run `38664958718` confirmed the asset-by-ID request is also denied (`HTTP 403:
Resource not accessible by integration`). The original-package scan remains
blocked by draft-asset access. No privilege escalation was deployed. The fresh
native build in this run passed Defender preparation and began compiling; its
eventual build/scan result must be checked before making further claims.

For isolated testing, the previously scanned native artifact was downloaded to
`dist/OpenPDF-Desktop-0.1.0-windows-x64-native-investigation.zip`. Its ZIP and EXE
hashes were checked against the reports above. Adjacent
`dist/windows-native-investigation-scan.json` records provenance and limitations.
This is a separate copy of the existing native CI artifact, not a relabeled or
modified copy of the flagged cross-build. It has not been recommended for use
with personal documents or production signing identities. Keep Defender enabled
in a disposable Windows VM and investigate any further detection; do not add an
exclusion. The unresolved distributed Windows package remains under investigation.

References:
- https://learn.microsoft.com/en-us/defender-endpoint/command-line-arguments-microsoft-defender-antivirus
- https://www.microsoft.com/en-us/wdsi/threats/malware-encyclopedia-description?name=Trojan%3AWin32%2FBearfoos.A%21ml
- https://www.microsoft.com/en-us/wdsi/filesubmission
