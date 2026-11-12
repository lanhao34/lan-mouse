"""Resolve official MSYS2 UCRT64 libraries into a Windows cross sysroot.
No Windows installers are run. Every archive is checked against repository SHA256.
"""
import hashlib, io, json, re, tarfile, time, urllib.request, subprocess
from pathlib import Path
BASE='https://repo.msys2.org/mingw/ucrt64/'
ROOT=Path('/opt/windows')
CACHE=Path('/tmp/msys-packages')
ROOT.mkdir(parents=True,exist_ok=True);CACHE.mkdir(parents=True,exist_ok=True)
def fetch(name):
    for n in range(3):
        try:
            with urllib.request.urlopen(BASE+name,timeout=90) as r:return r.read()
        except Exception:
            if n==2:raise
            time.sleep(2)
entries={}
db_bytes=fetch('ucrt64.db')
if db_bytes[:4]==bytes.fromhex('28b52ffd'):
    db_bytes=subprocess.run(['zstd','-d','--stdout'],input=db_bytes,capture_output=True,check=True).stdout
with tarfile.open(fileobj=io.BytesIO(db_bytes),mode='r:*') as db:
    for member in db.getmembers():
        if not member.name.endswith('/desc'):continue
        fields={};key=None
        for line in db.extractfile(member).read().decode().splitlines():
            if line.startswith('%') and line.endswith('%'):key=line.strip('%');fields[key]=[]
            elif line and key:fields[key].append(line)
        entries[fields['NAME'][0]]=fields
providers={}
for name,entry in entries.items():
    for alias in entry.get('PROVIDES',[]):providers.setdefault(re.split('[<>=]',alias)[0],name)
selected={}
def resolve(dep):
    name=re.split('[<>=]',dep)[0]
    name=name if name in entries else providers.get(name,name)
    if name in selected:return
    if name not in entries:raise RuntimeError('Missing dependency '+name)
    entry=entries[name];selected[name]=entry
    for dep in entry.get('DEPENDS',[]):resolve(dep)
for name in ['gtk4','libadwaita','librsvg']:resolve('mingw-w64-ucrt-x86_64-'+name)
manifest=[]
for name,entry in sorted(selected.items()):
    filename=entry['FILENAME'][0];expected=entry['SHA256SUM'][0];data=fetch(filename)
    if hashlib.sha256(data).hexdigest()!=expected:raise RuntimeError('Checksum mismatch '+filename)
    path=CACHE/filename;path.write_bytes(data)
    # Reject unexpected absolute/traversal archive members before extraction.
    members=subprocess.check_output(['tar','--zstd','-tf',str(path)],text=True).splitlines()
    if any(m.startswith('/') or '..' in Path(m).parts for m in members):raise RuntimeError('Unsafe archive path')
    subprocess.run(['tar','--zstd','-xf',str(path),'-C',str(ROOT)],check=True)
    manifest.append({'name':name,'version':entry['VERSION'][0],'file':filename,'sha256':expected})
    path.unlink();print('installed',name,entry['VERSION'][0],flush=True)
(ROOT/'packages.json').write_text(json.dumps(manifest,indent=2))
print('Windows sysroot ready:',len(manifest),'packages')
