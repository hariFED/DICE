@echo off
REM DICE firmware build wrapper.
REM Sources ESP-IDF v5.2.6 and runs `idf.py build` from firmware/.
REM
REM Usage (from repo root):
REM     firmware\build_firmware.bat
REM Or from firmware/:
REM     build_firmware.bat

setlocal

REM Resolve the firmware directory (this script lives inside it).
set FIRMWARE_DIR=%~dp0
if "%FIRMWARE_DIR:~-1%"=="\" set FIRMWARE_DIR=%FIRMWARE_DIR:~0,-1%

REM Clear Git Bash / MSYS markers before calling export.bat, otherwise
REM export.bat detects MSYSTEM and refuses to run.
set MSYSTEM=
set MSYS=
set MINGW_PREFIX=

REM Pin the ESP-IDF Python env explicitly. Without this, export.bat
REM derives the env path from the system python's version (e.g. 3.14)
REM and fails because only the 3.11 env was installed.
set IDF_PYTHON_ENV_PATH=C:\Espressif\python_env\idf5.2_py3.11_env

REM Source ESP-IDF environment.
call C:\Espressif\frameworks\esp-idf-v5.2.6\export.bat
if errorlevel 1 (
    echo.
    echo ERROR: ESP-IDF export.bat failed.
    exit /b 1
)

REM Move into the firmware directory and build.
cd /d "%FIRMWARE_DIR%"
echo.
echo Building DICE firmware in: %CD%
echo.
idf.py build
exit /b %errorlevel%
