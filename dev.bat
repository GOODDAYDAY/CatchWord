@echo off
title CatchWord Dev
echo Starting CatchWord...
set PATH=%USERPROFILE%\.cargo\bin;%PATH%
cd /d "%~dp0app"
npm run tauri dev
pause
