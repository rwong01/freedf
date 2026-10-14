#!/usr/bin/env python3
"""Build-time download only. Runtime never downloads code or documents."""
import argparse,hashlib,io,json,pathlib,tarfile,urllib.request
root=pathlib.Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('platform',choices=['linux-x64','win-x64','mac-x64','mac-arm64']);a=p.parse_args()
lock=json.loads((root/'packaging/pdfium.lock.json').read_text());entry=lock['assets'].get(a.platform)
if not entry:raise SystemExit('No reviewed checksum for this platform yet; do not package an unverified runtime')
url=f"https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F{lock['revision']}/pdfium-{a.platform}.tgz"
with urllib.request.urlopen(url,timeout=120) as r:data=r.read(50*1024*1024)
if hashlib.sha256(data).hexdigest()!=entry:raise SystemExit('PDFium archive SHA-256 mismatch')
destination=root/'vendor'/('pdfium' if a.platform=='linux-x64' else 'pdfium-'+a.platform.split('-')[0]+('-arm64' if a.platform=='mac-arm64' else ''))
destination.mkdir(parents=True,exist_ok=True)
with tarfile.open(fileobj=io.BytesIO(data),mode='r:gz') as t:t.extractall(destination,filter='data')
print(destination)
