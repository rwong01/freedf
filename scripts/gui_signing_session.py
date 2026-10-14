#!/usr/bin/env python3
"""Interactive acceptance harness: real GUI, GTK portal and protected Secret Service.
Run under an isolated dbus-run-session and Xvfb. Complete signing in each of the
two windows, closing the first to test identity reuse in the second process.
Private test identities are removed when the harness exits; PDFs saved via the UI
remain at the paths explicitly chosen in the native Save dialog.
"""
import json,os,pathlib,secrets,subprocess,tempfile,time
root=pathlib.Path(__file__).resolve().parents[1]
assert os.environ.get('DISPLAY') not in [None,':0'],'Use an isolated Xvfb display'
assert os.environ.get('DBUS_SESSION_BUS_ADDRESS'),'Run under dbus-run-session'
bin_dir=pathlib.Path(os.environ.get('PORTAL_BIN_DIR','/usr/libexec'))
with tempfile.TemporaryDirectory(prefix='openpdf-gui-sign-') as folder:
 env=os.environ.copy();env.pop('WAYLAND_DISPLAY',None)
 env.update(XDG_DATA_HOME=folder+'/data',XDG_CONFIG_HOME=folder+'/config',XDG_RUNTIME_DIR=folder+'/run',XDG_CURRENT_DESKTOP='GNOME',LIBGL_ALWAYS_SOFTWARE='1')
 pathlib.Path(env['XDG_RUNTIME_DIR']).mkdir(mode=0o700)
 config=pathlib.Path(env['XDG_CONFIG_HOME'])/'xdg-desktop-portal';config.mkdir(parents=True)
 (config/'portals.conf').write_text('[preferred]\ndefault=gtk\n')
 processes=[]
 try:
  keyring=subprocess.Popen([os.environ.get('KEYRING_DAEMON','gnome-keyring-daemon'),'--foreground','--unlock','--components=secrets','--control-directory',folder+'/control'],stdin=subprocess.PIPE,env=env)
  processes.append(keyring);keyring.stdin.write(secrets.token_bytes(32).hex().encode());keyring.stdin.close()
  for name in ['xdg-desktop-portal-gtk','xdg-desktop-portal']:
   processes.append(subprocess.Popen([str(bin_dir/name)],env=env));time.sleep(1)
  fingerprints=[]
  for n,fixture in enumerate(['forms.pdf','edited.pdf'],1):
   print(f'Opening signing window {n}; sign through the UI, then close it to continue.',flush=True)
   proc=subprocess.Popen([os.environ.get('OPENPDF_TEST_BINARY',str(root/'target/release/openpdf-app')),str(root/'fixtures'/fixture)],env=env);processes.append(proc)
   code=proc.wait();assert code==0,f'Application exit {code}'
   identities=list(pathlib.Path(env['XDG_DATA_HOME']).rglob('identity.json'))
   assert len(identities)==1,'Complete identity setup and signing before closing the window'
   fingerprints.append(json.loads(identities[0].read_text())['info']['fingerprint'])
  assert fingerprints[0]==fingerprints[1],'Identity changed between application launches'
  (root/'docs/evidence/gui-identity-reuse.txt').write_text('Two actual GUI application processes used the same protected test identity.\n'+fingerprints[0]+'\n')
  print('PASS: same protected identity across two GUI processes; independently verify both saved PDFs.',flush=True)
 finally:
  for proc in reversed(processes):
   if proc.poll() is None:
    proc.terminate()
    try:proc.wait(timeout=5)
    except subprocess.TimeoutExpired:proc.kill();proc.wait()
