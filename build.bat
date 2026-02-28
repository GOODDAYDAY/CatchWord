@echo off
title CatchWord Build
echo ========================================
echo   CatchWord - Production Build
echo ========================================
echo.

set PATH=%USERPROFILE%\.cargo\bin;%PATH%
cd /d "%~dp0app"

echo [1/2] Installing dependencies...
call npm install
if %errorlevel% neq 0 (
    echo ERROR: npm install failed
    pause
    exit /b 1
)

echo.
echo [2/2] Building Tauri application...
call npm run tauri build
if %errorlevel% neq 0 (
    echo ERROR: Build failed
    pause
    exit /b 1
)

echo.
echo ========================================
echo   Build complete!
echo   Output: app\src-tauri\target\release\bundle\
echo ========================================
explorer "app\src-tauri\target\release\bundle\nsis"
pause
