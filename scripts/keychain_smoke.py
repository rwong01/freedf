#!/usr/bin/env python3
"""Run under dbus-run-session; use an isolated, encrypted Secret Service keyring."""
import os,pathlib,secrets,subprocess,tempfile,time
root=pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='openpdf-keychain-test-') as temp:
    env=os.environ.copy();env['XDG_DATA_HOME']=temp+'/data';env['XDG_RUNTIME_DIR']=temp+'/run'
    pathlib.Path(env['XDG_RUNTIME_DIR']).mkdir(mode=0o700)
    daemon=os.environ.get('KEYRING_DAEMON','gnome-keyring-daemon')
    proc=subprocess.Popen([daemon,'--foreground','--unlock','--components=secrets','--control-directory',temp+'/control'],stdin=subprocess.PIPE,env=env)
    proc.stdin.write(secrets.token_bytes(32).hex().encode());proc.stdin.close()
    try:
        time.sleep(1)
        logs=[]
        for _ in range(2):
            logs.append(subprocess.check_output([os.environ.get('OPENPDF_TEST_BINARY',str(root/'target/debug/openpdf-app')),'--validate-keychain',temp+'/profile'],env=env,text=True))
        assert logs[0]==logs[1], 'Identity changed across application processes'
        import shutil
        shutil.copyfile(temp+'/profile/keychain-signed.pdf',root/'docs/evidence/keychain-signed.pdf')
        (root/'docs/evidence/keychain-test.txt').write_text('Two separate application processes, same isolated OS Secret Service.\n'+''.join(logs))
        print('PASS: persisted OS-protected identity reused across two application processes')
    finally:
        proc.terminate();proc.wait(timeout=10)
