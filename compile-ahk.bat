@echo off
REM KeyScroll - Compile AHK prototype to standalone EXE
REM Usage: compile.bat
REM Requires: AutoHotkey v2 installed or Ahk2Exe available

set AHK2EXE=Ahk2Exe.exe
set SCRIPT=%~dp0src-ahk\keyscroll.ahk
set OUTPUT=%~dp0keyscroll.exe

if exist "%AHK2EXE%" (
    "%AHK2EXE%" /in "%SCRIPT%" /out "%OUTPUT%"
) else (
    echo Ahk2Exe not found in PATH.
    echo Please install AutoHotkey v2 with Ahk2Exe, then run:
    echo   Ahk2Exe.exe /in "%SCRIPT%" /out "%OUTPUT%"
    exit /b 1
)

echo Compiled: %OUTPUT%