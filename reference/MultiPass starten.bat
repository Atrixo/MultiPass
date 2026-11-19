@echo off
rem MultiPass fuer Windows starten.
rem Tipp: NC-Dateien koennen auch direkt auf diese Datei gezogen werden.
setlocal
set "APP=%~dp0multipass.py"
set "PYW="

for %%P in (
  "%USERPROFILE%\anaconda3\pythonw.exe"
  "%USERPROFILE%\miniconda3\pythonw.exe"
  "%LOCALAPPDATA%\anaconda3\pythonw.exe"
  "%ProgramData%\anaconda3\pythonw.exe"
) do (
  if not defined PYW if exist %%P set "PYW=%%~P"
)

if not defined PYW (
  where pyw >nul 2>nul && set "PYW=pyw"
)
if not defined PYW (
  where pythonw >nul 2>nul && set "PYW=pythonw"
)

if not defined PYW (
  echo Python 3 wurde nicht gefunden.
  echo Bitte Python 3 von https://www.python.org installieren
  echo ^(beim Installieren "Add python.exe to PATH" anhaken^).
  pause
  exit /b 1
)

start "" "%PYW%" "%APP%" %*
endlocal
