#!/usr/bin/env python3
"""Independent Poppler/qpdf assertions over the generated synthetic corpus."""
import json,pathlib,subprocess
root=pathlib.Path(__file__).resolve().parents[1]
out=root/'docs/evidence';out.mkdir(parents=True,exist_ok=True)
report={}
def run(name,args):
 p=subprocess.run(args,cwd=root,text=True,capture_output=True)
 (out/(name+'.txt')).write_text(p.stdout+p.stderr)
 if p.returncode:raise RuntimeError(f'{name} exited {p.returncode}: {p.stderr}')
 return p.stdout
for file in ['forms','edited','signed','signed-twice']:
 run('qpdf-'+file,['qpdf','--check',f'fixtures/{file}.pdf']);report['qpdf-'+file]='passed syntax/stream check'
s=run('pdfsig-two-signatures',['pdfsig','-nocert','fixtures/signed-twice.pdf'])
assert s.count('Signature is Valid')==2,s
report['two-incremental-signatures']='both cryptographically valid; certificate trust not checked'
s=run('pdfsig-tampered',['pdfsig','-nocert','fixtures/tampered.pdf'])
assert 'Digest Mismatch' in s,s
report['tampering']='digest mismatch detected'
s=run('pdfsig-keychain',['pdfsig','-nocert','docs/evidence/keychain-signed.pdf'])
assert 'Signature is Valid' in s,s
report['os-keystore-signature']='cryptographically valid; certificate trust not checked'
s=run('poppler-field-text',['pdftotext','fixtures/edited.pdf','-'])
assert 'Alice Example' in s,s
report['persisted-form']='Poppler extracted Alice Example'
objects=json.loads(run('qpdf-edited-objects',['qpdf','--json','fixtures/edited.pdf']))
serialized=json.dumps(objects)
for kind in ['Highlight','Underline','StrikeOut','Ink','Text','FreeText','Square','Circle','Line','Stamp']:
 assert f'"/{kind}"' in serialized,kind
assert '"/SMask"' in serialized,'Transparent signature image mask missing'
assert '"/AcroForm"' in serialized,'Form structure was lost'
report['structures']='ten annotation kinds including transparent typed signature; AcroForm retained'
run('poppler-typed-signature-render',['pdftoppm','-f','3','-singlefile','-scale-to','1200','-png','fixtures/edited.pdf',str(out/'poppler-typed-signature')])
(out/'independent-validation.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
