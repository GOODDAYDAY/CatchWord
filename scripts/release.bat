@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion

:: Always run from repo root
cd /d "%~dp0.."

:: Get latest tag
for /f "delims=" %%i in ('git describe --tags --abbrev^=0 2^>nul') do set "latest=%%i"
if not defined latest set "latest=v0.0.0"

echo Current version: %latest%

:: Parse version (strip leading v)
set "ver=%latest:~1%"
for /f "tokens=1,2,3 delims=." %%a in ("%ver%") do (
    set "major=%%a"
    set "minor=%%b"
    set "patch=%%c"
)

:: Calculate next versions
set /a "next_patch=patch + 1"
set /a "next_minor=minor + 1"
set /a "next_major=major + 1"

echo.
echo   1) patch  -^> v%major%.%minor%.%next_patch%
echo   2) minor  -^> v%major%.%next_minor%.0
echo   3) major  -^> v%next_major%.0.0
echo.
set /p "choice=Choose [1/2/3]: "

if "%choice%"=="1" (
    set "new_ver=%major%.%minor%.%next_patch%"
) else if "%choice%"=="2" (
    set "new_ver=%major%.%next_minor%.0"
) else if "%choice%"=="3" (
    set "new_ver=%next_major%.0.0"
) else (
    echo Invalid choice.
    exit /b 1
)

set "new_tag=v!new_ver!"

:: Generate changelog from git log since last tag
echo.
echo ── Changelog (%latest% → !new_tag!) ──
echo.

set "tmpfile=%TEMP%\catchword_changelog.txt"
git log %latest%..HEAD --pretty=format:"- %%s" --no-merges > "!tmpfile!" 2>nul

:: Count commits
set "commit_count=0"
for /f %%n in ('git rev-list %latest%..HEAD --count 2^>nul') do set "commit_count=%%n"

if !commit_count! equ 0 (
    echo   No new commits since %latest%.
    echo.
    echo Nothing to release.
    pause
    exit /b 0
)

type "!tmpfile!"
echo.
echo   (%commit_count% commits)
echo.

set /p "confirm=Create and push tag !new_tag!? [y/N]: "

if /i not "%confirm%"=="y" (
    echo Cancelled.
    del "!tmpfile!" 2>nul
    exit /b 0
)

:: Sync version to tauri.conf.json and package.json via node
echo.
echo Updating version to !new_ver! ...
node -e "var fs=require('fs');['app/src-tauri/tauri.conf.json','app/package.json'].forEach(function(f){var j=JSON.parse(fs.readFileSync(f,'utf8'));j.version='!new_ver!';fs.writeFileSync(f,JSON.stringify(j,null,2)+'\n')})"

:: Build commit message with changelog
set "msgfile=%TEMP%\catchword_commit_msg.txt"
echo chore: bump version to !new_ver!> "!msgfile!"
echo.>> "!msgfile!"
echo Changes since %latest%:>> "!msgfile!"
type "!tmpfile!" >> "!msgfile!"

:: Commit version bump, tag, and push
git add app\src-tauri\tauri.conf.json app\package.json
git commit -F "!msgfile!"
git tag -a !new_tag! -F "!msgfile!"
git push origin HEAD !new_tag!

:: Cleanup
del "!tmpfile!" 2>nul
del "!msgfile!" 2>nul

echo.
echo Tag !new_tag! pushed. GitHub Actions will build the release.
pause
