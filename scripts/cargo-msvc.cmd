@echo off
setlocal

set "VSCMD_SKIP_SENDTELEMETRY=1"
set "VSDEVCMD=%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"

if not exist "%VSDEVCMD%" (
    echo error: Visual Studio Build Tools developer command script not found at "%VSDEVCMD%"
    exit /b 1
)

call "%VSDEVCMD%" -arch=x64 -host_arch=x64 >nul
if errorlevel 1 exit /b %errorlevel%

pushd "%~dp0..\rust" >nul
if errorlevel 1 (
    echo error: could not enter rust workspace
    exit /b 1
)

cargo %*
set "EXITCODE=%ERRORLEVEL%"

popd >nul
exit /b %EXITCODE%
