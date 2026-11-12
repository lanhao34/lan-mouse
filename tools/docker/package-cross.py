"""Assemble a portable Windows candidate without executing Windows binaries."""
from pathlib import Path
import hashlib, json, re, shutil, subprocess, zipfile
src=Path('/src');out=Path('/dist');prefix=Path('/opt/windows/ucrt64')
package=out/'lan-mouse-windows-x64-cross'
if package.exists():raise RuntimeError('Package directory already exists; preserve or move it before rebuilding')
package.mkdir()
for name in ['lan-mouse.exe','edge-settings-preview.exe']:shutil.copy2(out/name,package/name)
for path in (prefix/'bin').glob('*.dll'):shutil.copy2(path,package/path.name)
shutil.copy2(prefix/'bin/gdk-pixbuf-query-loaders.exe',package/'gdk-pixbuf-query-loaders.exe')
for folder in ['share/glib-2.0/schemas','share/icons/Adwaita','share/icons/hicolor','share/mime','share/licenses','lib/gdk-pixbuf-2.0']:
    if (prefix/folder).exists():shutil.copytree(prefix/folder,package/folder)
subprocess.run(['glib-compile-schemas',str(package/'share/glib-2.0/schemas')],check=True)
helper=src/'target/windows-edge-overlay'
if not (helper/'edge-overlay.exe').is_file():raise RuntimeError('Publish the native helper first on the host')
shutil.copytree(helper,package/'native-edge-overlay',ignore=shutil.ignore_patterns('*.pdb'))
shutil.copy2(src/'tools/launch-windows.cmd',package/'Launch.cmd')
shutil.copy2(src/'LICENSE',package/'LICENSE')
shutil.copy2(src/'docs/WINDOWS_TEST_BUILD.md',package/'WINDOWS_TEST_BUILD.md')
shutil.copy2('/opt/windows/packages.json',package/'windows-libraries.json')
revision=subprocess.check_output(['git','-C',str(src),'rev-parse','HEAD'],text=True).strip()
(package/'SOURCE_COMMIT.txt').write_text(revision+'\n')
(package/'CROSS_BUILD_STATUS.txt').write_text('Cross-compiled on Apple Silicon via Linux ARM64 Docker.\nRust Windows binaries linked successfully. Native Windows execution has NOT been validated for this package.\nRange configuration remains draft-only and does not change live cross-screen behavior.\nExtract all files into a new directory and use Launch.cmd preview first. Do not overwrite a working installation.\n')
# Record PE imports to make native acceptance checks auditable.
imports={}
for binary in [package/'lan-mouse.exe',package/'edge-settings-preview.exe']:
    data=subprocess.check_output(['x86_64-w64-mingw32-objdump','-p',str(binary)],text=True)
    imports[binary.name]=re.findall(r'DLL Name: (.+)',data)
(package/'PE_IMPORTS.json').write_text(json.dumps(imports,indent=2))
archive=out/f'lan-mouse-windows-x64-cross-{revision[:8]}.zip'
with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as z:
    for path in sorted(package.rglob('*')):
        if path.is_file():z.write(path,path.relative_to(package))
sha=hashlib.sha256(archive.read_bytes()).hexdigest()
archive.with_suffix('.zip.sha256').write_text(f'{sha}  {archive.name}\n')
print(archive.name,archive.stat().st_size,'bytes',sha)
