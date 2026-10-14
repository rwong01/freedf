#!/usr/bin/env python3
"""Inventory the exact Cargo.lock graph and copy distribution license texts."""
import json,pathlib,shutil,subprocess,os
root=pathlib.Path(__file__).resolve().parents[1];cargo=os.environ.get('CARGO','cargo')
meta=json.loads(subprocess.check_output([cargo,'metadata','--locked','--offline','--format-version','1'],cwd=root))
out=root/'dist/third-party';out.mkdir(parents=True,exist_ok=True)
rows=['# Third-party dependencies','', 'Generated from the locked dependency graph. Includes all target-specific and optional packages reported by Cargo.','', '| Package | Version | Declared license | Source |','|---|---|---|---|']
for package in sorted(meta['packages'],key=lambda p:(p['name'],p['version'])):
 if package['name'].startswith('openpdf-'):continue
 source=package.get('repository') or package.get('source') or ''
 rows.append(f"| {package['name']} | {package['version']} | {package.get('license') or 'REVIEW REQUIRED'} | {source} |")
 directory=pathlib.Path(package['manifest_path']).parent;dest=out/(package['name']+'-'+package['version']);dest.mkdir(exist_ok=True)
 copied=0
 for f in directory.iterdir():
  if f.is_file() and (f.name.upper().startswith(('LICENSE','COPYING','NOTICE','COPYRIGHT'))):shutil.copy2(f,dest/f.name);copied+=1
 license_file=package.get('license_file')
 if license_file and (directory/license_file).exists():shutil.copy2(directory/license_file,dest/pathlib.Path(license_file).name);copied+=1
 # Some upstream crates include only SPDX metadata. Retain their exact manifest for review.
 shutil.copy2(directory/'Cargo.toml',dest/'Cargo.toml')
 if package['name']=='pdf_signer':shutil.copy2(root/'LICENSE',dest/'GPL-3.0.txt')
(root/'docs/THIRD_PARTY_NOTICES.md').write_text('\n'.join(rows)+'\n\nPDFium binary archives carry their own LICENSE and licenses directory; all are bundled verbatim.\nCaveat and Dancing Script font licenses are in assets/fonts. Original OpenPDF icon is GPL-3.0-or-later.\nA dependency inventory is not a complete legal audit; missing upstream notices must be reviewed before a public release.\n')
print(f'Inventoried {len(meta["packages"])} packages')
