# Run in Windows PowerShell or pwsh from a clean repository root.
# No Actions runner, paid service, firewall changes, or installed app replacement.
[CmdletBinding()]
param(
    [string]$MsysRoot = 'C:\msys64',
    [switch]$InstallBuildDependencies,
    [switch]$Publish
)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
if ((Resolve-Path .).Path -ne (Resolve-Path $repoRoot).Path) { throw 'Run this script from the repository root.' }
foreach ($tool in @('git','dotnet')) { if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "Missing $tool. Install the official Windows tool before building." } }
$bash = Join-Path $MsysRoot 'usr\bin\bash.exe'
if (-not (Test-Path $bash)) { throw "MSYS2 not found at $MsysRoot. Install MSYS2 from https://www.msys2.org/ or pass -MsysRoot." }
$sha = (& git rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Could not determine source commit.' }
if (& git status --porcelain) { throw 'Use a clean checkout so the package matches its recorded source commit.' }
if ($Publish) {
    if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { throw 'Install GitHub CLI or omit -Publish and upload the ZIP later.' }
    & gh auth status
    if ($LASTEXITCODE -ne 0) { throw 'GitHub CLI is not signed in. Build without -Publish until sign-in is complete.' }
}
$oldEnvironment = @{}
$names = @('MSYSTEM','CHERE_INVOKING','INSTALL_BUILD_DEPS','LOCAL_BUILD_SCRIPT','PATH','XDG_DATA_DIRS','GSETTINGS_SCHEMA_DIR','GDK_PIXBUF_MODULEDIR')
foreach ($name in $names) { $oldEnvironment[$name] = [Environment]::GetEnvironmentVariable($name,'Process') }
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
New-Item -ItemType Directory -Force target/manual-build-logs | Out-Null
Start-Transcript -Path "target/manual-build-logs/$stamp.log" | Out-Null
try {
    Write-Host "Building source $sha; the installed Lan Mouse is not changed."
    & dotnet publish lan-mouse-gtk/native/windows/EdgeOverlay.csproj -c Release -r win-x64 --self-contained true -o target/windows-edge-overlay
    if ($LASTEXITCODE -ne 0) { throw 'Native overlay publish failed.' }
    & target/windows-edge-overlay/edge-overlay.exe --self-test
    if ($LASTEXITCODE -ne 0) { throw 'Native geometry tests failed.' }
    & target/windows-edge-overlay/edge-overlay.exe --smoke-test
    if ($LASTEXITCODE -ne 0) { throw 'Native window smoke test failed.' }
    $env:MSYSTEM = 'UCRT64'
    $env:CHERE_INVOKING = '1'
    $env:INSTALL_BUILD_DEPS = if ($InstallBuildDependencies) { '1' } else { '0' }
    $env:LOCAL_BUILD_SCRIPT = (Join-Path $repoRoot 'tools\build-windows-local.sh').Replace('\','/')
    & $bash --login -c 'exec /usr/bin/bash "$LOCAL_BUILD_SCRIPT"'
    if ($LASTEXITCODE -ne 0) { throw 'Rust build/tests or packaging failed. See transcript.' }
    $package = (Resolve-Path dist/lan-mouse-windows-x64).Path
    $env:PATH = "$package;$env:SystemRoot\System32;$env:SystemRoot"
    $env:XDG_DATA_DIRS = "$package\share"
    $env:GSETTINGS_SCHEMA_DIR = "$package\share\glib-2.0\schemas"
    $env:GDK_PIXBUF_MODULEDIR = "$package\lib\gdk-pixbuf-2.0\2.10.0\loaders"
    & "$package\gdk-pixbuf-query-loaders.exe" | Out-File -Encoding utf8 "$package\loaders.cache"
    if ($LASTEXITCODE -ne 0) { throw 'Packaged image loader failed.' }
    & "$package\lan-mouse.exe" --version
    if ($LASTEXITCODE -ne 0) { throw 'Packaged application could not load its runtime DLLs.' }
    & "$package\native-edge-overlay\edge-overlay.exe" --self-test
    if ($LASTEXITCODE -ne 0) { throw 'Packaged overlay tests failed.' }
    Remove-Item "$package\loaders.cache"
    $env:PATH = $oldEnvironment['PATH']
    $archive = "dist/lan-mouse-windows-x64-$($sha.Substring(0,8))-$stamp.zip"
    Compress-Archive -Path "$package\*" -DestinationPath $archive
    $hash = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLower()
    $hashFile = "$archive.sha256"
    "$hash  $(Split-Path -Leaf $archive)" | Out-File -Encoding ascii $hashFile
    Write-Host "BUILD COMPLETE: $archive"
    Write-Host "SHA256: $hash"
    if ($Publish) {
        $tag = "dev-windows-manual-$stamp-$($sha.Substring(0,8))"
        & gh release create $tag $archive $hashFile --repo lanhao34/lan-mouse --target $sha --prerelease --latest=false --title "Windows development $($sha.Substring(0,8)) (manual)" --notes-file docs/WINDOWS_TEST_BUILD.md
        if ($LASTEXITCODE -ne 0) { throw "Upload failed; local package is preserved at $archive. Do not rebuild merely to retry upload." }
    }
} finally {
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name,$oldEnvironment[$name],'Process') }
    Stop-Transcript | Out-Null
}
