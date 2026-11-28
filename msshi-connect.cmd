@echo off
setlocal EnableExtensions
set "SCRIPT_DIR=%~dp0"
if exist "%SCRIPT_DIR%MSSHI-Connect.exe" goto portable
if exist "%SCRIPT_DIR%target\release\msshi-connect.exe" goto build
node "%SCRIPT_DIR%dev\node-bridge.mjs" %*
exit /b %ERRORLEVEL%
:portable
"%SCRIPT_DIR%MSSHI-Connect.exe" %*
exit /b %ERRORLEVEL%
:build
"%SCRIPT_DIR%target\release\msshi-connect.exe" %*
exit /b %ERRORLEVEL%
