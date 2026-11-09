@echo off
setlocal
set "XDG_DATA_DIRS=%~dp0share"
set "GSETTINGS_SCHEMA_DIR=%~dp0share\glib-2.0\schemas"
set "GDK_PIXBUF_MODULEDIR=%~dp0lib\gdk-pixbuf-2.0\2.10.0\loaders"
set "GDK_PIXBUF_MODULE_FILE=%~dp0loaders.cache"
"%~dp0gdk-pixbuf-query-loaders.exe" > "%GDK_PIXBUF_MODULE_FILE%"
if /I "%~1"=="preview" (
  start "" "%~dp0edge-settings-preview.exe"
) else (
  start "" "%~dp0lan-mouse.exe"
)
endlocal
