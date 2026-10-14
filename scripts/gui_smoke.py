#!/usr/bin/env python3
"""Drive the real 1280x900 application on a dedicated Xvfb display.
Requires xdotool/scrot. Never run against a user's interactive display.
Start OpenPDF with a disposable copy of fixtures/forms.pdf before this script.
"""
import os, pathlib, subprocess, time, argparse
p=argparse.ArgumentParser();p.add_argument('phase',choices=['forms','search','annotation','draw','capture','copy']);p.add_argument('--output',default='docs/evidence/gui-test.png');a=p.parse_args()
if os.environ.get('DISPLAY') in [None, ':0']:
    raise SystemExit('Use a dedicated virtual display, not the user desktop')
xd=os.environ.get('XDOTOOL','xdotool');sc=os.environ.get('SCROT','scrot')
def run(*args):return subprocess.check_output([xd,*map(str,args)],text=True).strip()
def click(x,y):run('mousemove',x,y,'click',1);time.sleep(.2)
def key(k):run('key','--clearmodifiers',k);time.sleep(.2)
def type_text(s):run('type','--clearmodifiers','--delay','20',s);time.sleep(.2)
windows=run('search','--name','^OpenPDF Desktop$').splitlines()
if len(windows)!=1:raise SystemExit(f'Expected one isolated application window; got {windows}')
w=windows[0];run('windowsize',w,1280,900);run('windowmove',w,0,0);run('windowfocus',w);time.sleep(.4)
if a.phase=='forms':
    click(460,480);key('ctrl+a');type_text('GUI Alice');click(335,674);key('ctrl+s');time.sleep(2)
elif a.phase=='search':
    click(432,50);click(120,185);type_text('Searchable');time.sleep(2)
elif a.phase=='annotation':
    click(580,155);run('mousemove',500,700,'mousedown',1);time.sleep(.15)
    for x,y in [(530,710),(570,740),(620,770)]:run('mousemove',x,y);time.sleep(.1)
    run('mouseup',1);time.sleep(.3);key('ctrl+s');time.sleep(2)
elif a.phase=='copy':
    click(40,155);run('mousemove',325,380,'mousedown',1);time.sleep(.2)
    for x,y in [(340,385),(430,390),(600,402)]:run('mousemove',x,y);time.sleep(.2)
    run('mouseup',1);time.sleep(.3);key('ctrl+c')
elif a.phase=='draw':
    click(788,155);time.sleep(.5)
path=pathlib.Path(a.output);path.parent.mkdir(parents=True,exist_ok=True)
subprocess.run([sc,'--overwrite',str(path)],check=True)
print(f'GUI input phase {a.phase} executed; screenshot: {path}. Inspect screenshot and saved PDF before claiming acceptance.')
