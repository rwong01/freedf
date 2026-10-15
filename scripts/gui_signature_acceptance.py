#!/usr/bin/env python3
"""Real signature UI regression on an isolated 1280x900 Xvfb app/profile.
The shell harness creates the PDF/profile. No keyboard input targets a user desktop.
"""
import argparse, json, math, os, pathlib, shutil, subprocess, time
p=argparse.ArgumentParser();p.add_argument('pdf',type=pathlib.Path);p.add_argument('--reopen',action='store_true');a=p.parse_args()
assert os.environ.get('DISPLAY') not in [None,':0'], 'Use a dedicated virtual display'
out=pathlib.Path('docs/evidence/signature-ux');out.mkdir(parents=True,exist_ok=True)
xd=os.environ.get('XDOTOOL','xdotool')
def run(*args): return subprocess.check_output([xd,*map(str,args)],text=True).strip()
def click(x,y):
    run('mousemove',x,y);time.sleep(.15);run('click',1);time.sleep(.4)
def key(k): run('key','--clearmodifiers',k);time.sleep(.3)
def shot(name):
    time.sleep(.4);subprocess.run(['scrot','--overwrite',str(out/(name+'.png'))],check=True)
def drag(points):
    run('mousemove',*points[0]);time.sleep(.2);run('mousedown',1);time.sleep(.2)
    for start,end in zip(points,points[1:]):
        for i in range(1,11):
            run('mousemove',int(start[0]+(end[0]-start[0])*i/10),int(start[1]+(end[1]-start[1])*i/10));time.sleep(.07)
    run('mouseup',1);time.sleep(.4)
def annotations():
    data=json.loads(subprocess.check_output(['qpdf','--json',str(a.pdf)],text=True))
    objects=data['qpdf'][1];result=[]
    for page in data['pages']:
        refs=objects['obj:'+page['object']]['value'].get('/Annots',[])
        if isinstance(refs,str):refs=objects['obj:'+refs]['value']
        for ref in refs:
            value=objects['obj:'+ref]['value'].get('/OpenPDFData')
            if value:
                assert value.startswith('u:');result.append(json.loads(value[2:]))
    return result

def save(name):
    key('ctrl+s');time.sleep(1.2)
    subprocess.run(['qpdf','--check',str(a.pdf)],check=True,stdout=subprocess.DEVNULL)
    shutil.copyfile(a.pdf,out/(name+'.pdf'));shot(name)
    return annotations()
def size(annotation):
    r=annotation['rect'];return r['right']-r['left'],r['top']-r['bottom']
windows=run('search','--name','^OpenPDF Desktop$').splitlines();assert len(windows)==1
run('windowsize',windows[0],1280,900);run('windowmove',windows[0],0,0);run('windowfocus',windows[0]);time.sleep(1.)
if a.reopen:
    assert len(annotations())==3
    click(1000,720);shot('reopened-selected')
    print('Reopened saved signature PDF in actual UI; screenshot retained.');raise SystemExit
click(500,155);click(438,276);click(490,371)
run('type','--clearmodifiers','--delay','25','Alex Morgan');time.sleep(.3)
click(365,552);shot('create-signature');click(420,634);click(1000,620)
original=save('auto-placed');assert len(original)==1 and original[0]['signature']
w,h=size(original[0]);assert 140<w<160 and h>10
# One corner gesture produces one undo step, with proportional scaling.
drag([(1128,649),(1020,624)])
resized=save('corner-resized');rw,rh=size(resized[0]);assert .4*w<rw<.8*w
assert math.isclose(rw/rh,w/h,rel_tol=1e-4)
key('ctrl+z');undone=save('resize-undone')
assert all(math.isclose(undone[0]['rect'][k],original[0]['rect'][k],abs_tol=.001) for k in original[0]['rect'])
key('ctrl+y');click(315,190);auto=save('auto-restored')
assert math.isclose(size(auto[0])[0],w,rel_tol=1e-4)
# A remembered signature has a preview/name and can fit a user-drawn box.
click(500,155);click(635,303);shot('saved-signatures');click(500,536)
drag([(900,730),(1100,780)])
fitted=save('drag-fitted');assert len(fitted)==2 and fitted[1]['signature']
fw,fh=size(fitted[1]);assert 110<fw<125 and fh<31
assert math.isclose(fw/fh,w/h,rel_tol=1e-4)
# Drawn signatures also crop unused canvas space and persist as native ink.
click(500,155);click(382,389)
drag([(420,400),(440,365),(430,470),(480,440),(520,420),(550,450)])
click(420,662);click(1100,450)
drawn=save('drawn-signature');assert len(drawn)==3
ink=drawn[2];assert ink['kind']=='Ink' and ink['signature']
assert len(ink['strokes'][0])>10
r=ink['rect'];assert all(r['left']<=pt['x']<=r['right'] and r['bottom']<=pt['y']<=r['top'] for pt in ink['strokes'][0])
click(635,155);shot('certificate-explanation');key('Escape')
report={'automatic_points':[w,h],'resized_points':[rw,rh],'drag_fitted_points':[fw,fh],
        'checks':['typed placement','remember and reuse','corner resize','single-step undo','redo','auto size','drag to fit','drawn ink','PDF save and qpdf structure'],
        'certificate_dialog':'visually inspected; this test does not submit certificate signing'}
(out/'results.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
