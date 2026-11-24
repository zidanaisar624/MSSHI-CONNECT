@echo off
setlocal EnableExtensions
set "SCRIPT_DIR=%~dp0"
if exist "D:\HermesData" (
  if not exist "D:\HermesData\temp" mkdir "D:\HermesData\temp"
  set "TEMP=D:\HermesData\temp"
  set "TMP=D:\HermesData\temp"
)
if exist "%SCRIPT_DIR%MSSHI-Connect.exe" (
  start "" "%SCRIPT_DIR%MSSHI-Connect.exe" --gui %*
  exit /b 0
)
if exist "%SCRIPT_DIR%target\release\msshi-connect.exe" (
  start "" "%SCRIPT_DIR%target\release\msshi-connect.exe" --gui %*
  exit /b 0
)
where node >nul 2>nul
if errorlevel 1 (
  echo [ERROR] MSSHI-Connect.exe maupun Node.js belum ditemukan. Pasang Node.js 20+ atau unduh paket portabel Windows x64 dari GitHub Actions.
  pause
  exit /b 1
)
node "%SCRIPT_DIR%dev\node-bridge.mjs" %*
