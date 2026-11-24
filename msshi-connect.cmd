@echo off
setlocal EnableExtensions
set "SCRIPT_DIR=%~dp0"
if exist "%SCRIPT_DIR%MSSHI-Connect.exe" (
  "%SCRIPT_DIR%MSSHI-Connect.exe" %*
  exit /b %ERRORLEVEL%
)
if exist "%SCRIPT_DIR%target\release\msshi-connect.exe" (
  "%SCRIPT_DIR%target\release\msshi-connect.exe" %*
  exit /b %ERRORLEVEL%
)
node "%SCRIPT_DIR%dev\node-bridge.mjs" %*
exit /b %ERRORLEVEL%
