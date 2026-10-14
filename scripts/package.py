#!/usr/bin/env python3
"""Stage standalone artifacts without silently omitting PDFium or license notices."""
import argparse,pathlib,shutil,subprocess,tarfile,zipfile,json,hashlib,plistlib
root=pathlib.Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('platform',choices=['linux-x64','windows-x64','mac-x64','mac-arm64']);p.add_argument('--binary',type=pathlib.Path,required=True);a=p.parse_args()
binary=a.binary.resolve();assert binary.is_file(),binary
out=root/'dist'/a.platform;out.mkdir(parents=True,exist_ok=True)
win=a.platform=='windows-x64';mac=a.platform.startswith('mac-')
runtime=root/'vendor'/('pdfium-win' if win else ('pdfium-mac-arm64' if a.platform=='mac-arm64' else 'pdfium-mac') if mac else 'pdfium')
lib=runtime/('bin/pdfium.dll' if win else 'lib/libpdfium.dylib' if mac else 'lib/libpdfium.so');assert lib.is_file(),lib
if mac:
 app=out/'OpenPDF Desktop.app'/'Contents';payload=app/'MacOS';payload.mkdir(parents=True,exist_ok=True);resources=app/'Resources';resources.mkdir(exist_ok=True)
 info={'CFBundleName':'OpenPDF Desktop','CFBundleDisplayName':'OpenPDF Desktop','CFBundleIdentifier':'org.openpdf.Desktop','CFBundleVersion':'0.1.0','CFBundleShortVersionString':'0.1.0','CFBundleExecutable':'openpdf','CFBundlePackageType':'APPL','NSHighResolutionCapable':True,'LSMinimumSystemVersion':'12.0','CFBundleDocumentTypes':[{'CFBundleTypeName':'PDF document','CFBundleTypeRole':'Editor','LSItemContentTypes':['com.adobe.pdf'],'LSHandlerRank':'Alternate'}]}
 (app/'Info.plist').write_bytes(plistlib.dumps(info))
else:payload=out
shutil.copy2(binary,payload/('openpdf.exe' if win else 'openpdf'));shutil.copy2(lib,payload/lib.name)
shutil.copy2(root/'LICENSE',out/'LICENSE');shutil.copy2(root/'README.md',out/'README.md')
shutil.copytree(root/'docs',out/'docs',dirs_exist_ok=True,ignore=shutil.ignore_patterns('evidence'))
notices=out/'licenses';notices.mkdir(exist_ok=True);shutil.copy2(runtime/'LICENSE',notices/'PDFium-LICENSE');shutil.copytree(runtime/'licenses',notices/'pdfium',dirs_exist_ok=True)
assert (root/'dist/third-party').is_dir(),'Run scripts/third_party.py first'
shutil.copytree(root/'dist/third-party',notices/'rust',dirs_exist_ok=True)
shutil.copytree(root/'assets/fonts',notices/'fonts',dirs_exist_ok=True)
shutil.copy2(root/'assets/icon.ico',out/'icon.ico')
if win:
 lines=[]
 for f in sorted(out.rglob('*')):
  if f.is_file():lines.append('Delete "$INSTDIR\\'+str(f.relative_to(out)).replace('/','\\')+'"')
 for d in sorted((f for f in out.rglob('*') if f.is_dir()),key=lambda p:len(p.parts),reverse=True):lines.append('RMDir "$INSTDIR\\'+str(d.relative_to(out)).replace('/','\\')+'"')
 (root/'dist/windows-uninstall.nsh').write_text('\n'.join(lines)+'\n')
 with zipfile.ZipFile(root/'dist/OpenPDF-Desktop-0.1.0-windows-x64.zip','w',zipfile.ZIP_DEFLATED,strict_timestamps=False) as z:
  for f in out.rglob('*'):
   if f.is_file():z.write(f,f.relative_to(out.parent))
elif not mac:
 stage=root/'dist/deb-root';app=stage/'opt/openpdf';shutil.copytree(out,app,dirs_exist_ok=True)
 binpath=stage/'usr/bin';binpath.mkdir(parents=True,exist_ok=True);launcher=binpath/'openpdf';launcher.write_text('#!/bin/sh\nexec /opt/openpdf/openpdf "$@"\n');launcher.chmod(0o755)
 desktop=stage/'usr/share/applications';desktop.mkdir(parents=True,exist_ok=True);shutil.copy2(root/'packaging/linux/org.openpdf.Desktop.desktop',desktop)
 icons=stage/'usr/share/icons/hicolor/256x256/apps';icons.mkdir(parents=True,exist_ok=True);shutil.copy2(root/'assets/icon.png',icons/'org.openpdf.Desktop.png')
 control=stage/'DEBIAN';control.mkdir(exist_ok=True);(control/'control').write_text('Package: openpdf-desktop\nVersion: 0.1.0\nSection: graphics\nPriority: optional\nArchitecture: amd64\nMaintainer: OpenPDF contributors <openpdf@localhost>\nDepends: libc6 (>= 2.39), libx11-6, libxkbcommon0, libxkbcommon-x11-0, libgl1, libegl1, libwayland-client0\nRecommends: gnome-keyring, cups-client, xdg-desktop-portal, xdg-desktop-portal-gtk\nDescription: Offline PDF reader and signer (development release)\n')
 subprocess.run(['dpkg-deb','--root-owner-group','--build',str(stage),str(root/'dist/OpenPDF-Desktop-0.1.0-linux-x64.deb')],check=True)
if not win:
 with tarfile.open(root/f'dist/OpenPDF-Desktop-0.1.0-{a.platform}.tar.gz','w:gz') as tar:tar.add(out,arcname=out.name)
manifest={str(f.relative_to(out)):hashlib.sha256(f.read_bytes()).hexdigest() for f in out.rglob('*') if f.is_file()};(root/f'dist/{a.platform}-SHA256.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(out)
