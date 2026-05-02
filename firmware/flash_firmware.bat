@echo off
REM DICE firmware flash wrapper.
REM Sources ESP-IDF v5.2.6 and runs `idf.py -p %1 flash`.
REM
REM Usage (from repo root):
REM     firmware\flash_firmware.bat COM7
REM Or from firmware/:
REM     flash_firmware.bat COM7

setlocal

if "%1"=="" (
    echo ERROR: serial port required
    echo Usage: flash_firmware.bat COM7
    exit /b 1
)

set PORT=%1

REM Resolve the firmware directory (this script lives inside it).
set FIRMWARE_DIR=%~dp0
if "%FIRMWARE_DIR:~-1%"=="\" set FIRMWARE_DIR=%FIRMWARE_DIR:~0,-1%

REM Clear Git Bash / MSYS markers before calling export.bat.
set MSYSTEM=
set MSYS=
set MINGW_PREFIX=

REM Pin the ESP-IDF Python env explicitly.
set IDF_PYTHON_ENV_PATH=C:\Espressif\python_env\idf5.2_py3.11_env

REM Source ESP-IDF environment.
call C:\Espressif\frameworks\esp-idf-v5.2.6\export.bat
if errorlevel 1 (
    echo.
    echo ERROR: ESP-IDF export.bat failed.
    exit /b 1
)

REM Move into the firmware directory and flash.
cd /d "%FIRMWARE_DIR%"
echo.
echo Flashing DICE firmware to %PORT%
echo.
idf.py -p %PORT% flash
exit /b %errorlevel%
