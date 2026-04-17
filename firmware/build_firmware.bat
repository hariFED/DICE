@echo off
setlocal
set MSYSTEM=
set MSYS=
set MINGW_PREFIX=
set IDF_PYTHON_ENV_PATH=C:\Espressif\python_env\idf5.2_py3.11_env
call C:\Espressif\frameworks\esp-idf-v5.2.6\export.bat
if errorlevel 1 (
    echo ERROR: ESP-IDF export.bat failed.
    exit /b 1
)
cd /d "%~dp0"
idf.py build
exit /b %errorlevel%
