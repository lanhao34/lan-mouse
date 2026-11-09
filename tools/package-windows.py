"""Package the UCRT64 build and recursively resolved runtime libraries."""
import os
from pathlib import Path
import re
import shutil
import subprocess

root = Path.cwd()
prefix = Path(os.environ['GTK_PREFIX']).resolve()
dest = root / 'dist' / 'lan-mouse-windows-x64'
dest.mkdir(parents=True, exist_ok=True)

sources = [root/'target/release/lan-mouse.exe', root/'target/release/examples/edge-selector.exe', prefix/'bin/gdk-pixbuf-query-loaders.exe']
for source, name in zip(sources, ['lan-mouse.exe', 'edge-settings-preview.exe', 'gdk-pixbuf-query-loaders.exe']):
    shutil.copy2(source, dest/name)

for folder in ['share/glib-2.0/schemas', 'share/icons/Adwaita', 'share/icons/hicolor', 'share/mime', 'share/licenses', 'lib/gdk-pixbuf-2.0']:
    source = prefix/folder
    if source.exists(): shutil.copytree(source, dest/folder, dirs_exist_ok=True)

roots = sources + list((prefix/'lib/gdk-pixbuf-2.0').rglob('*.dll'))
needed = set()
for binary in roots:
    result = subprocess.run([str(prefix/'bin/ntldd.exe'), '-R', str(binary)], check=True, text=True, capture_output=True)
    for line in result.stdout.splitlines():
        if '=> not found' in line and not re.search(r'(api-ms-|ext-ms-)', line, re.I):
            raise RuntimeError(f'Unresolved runtime dependency: {line}')
        match = re.search(r'=>\s+(.+?)\s+\(0x[0-9a-fA-F]+\)', line)
        if match:
            path = Path(match[1].strip()).resolve()
            if path.is_relative_to(prefix): needed.add(path)
for path in sorted(needed): shutil.copy2(path, dest/path.name)

shutil.copytree(root/'target/windows-edge-overlay',dest/'native-edge-overlay',dirs_exist_ok=True)
for file in ['LICENSE', 'docs/WINDOWS_TEST_BUILD.md']:
    shutil.copy2(root/file,dest/Path(file).name)
(dest/'SOURCE_COMMIT.txt').write_text(subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()+'\n',encoding='utf-8')
(dest/'RUNTIME_PACKAGES.txt').write_text(subprocess.check_output(['pacman','-Q'],text=True),encoding='utf-8')
print(f'Packaged {len(needed)} runtime DLLs in {dest}')
