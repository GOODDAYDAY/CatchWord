@echo off
setlocal enabledelayedexpansion

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
    set "new_tag=v%major%.%minor%.%next_patch%"
) else if "%choice%"=="2" (
    set "new_tag=v%major%.%next_minor%.0"
) else if "%choice%"=="3" (
    set "new_tag=v%next_major%.0.0"
) else (
    echo Invalid choice.
    exit /b 1
)

echo.
echo %latest% -^> !new_tag!
echo.
set /p "confirm=Create and push tag !new_tag!? [y/N]: "

if /i not "%confirm%"=="y" (
    echo Cancelled.
    exit /b 0
)

git tag !new_tag!
git push origin !new_tag!

echo.
echo Tag !new_tag! pushed. GitHub Actions will build the release.
