@echo off
setlocal

if "%~2"=="" (
    echo usage: %~nx0 ^<game_dir^> ^<dropfile_path^> [extra nc-game args...] 1>&2
    exit /b 64
)

set "GAME_DIR=%~1"
set "DROPFILE=%~2"
shift
shift

set "EXTRA_ARGS="
:collect_args
if "%~1"=="" goto args_done
set "EXTRA_ARGS=%EXTRA_ARGS% "%~1""
shift
goto collect_args
:args_done

for %%I in ("%~dp0..\..") do set "REPO_ROOT=%%~fI"
set "RUST_DIR=%REPO_ROOT%\rust"
set "RELEASE_BIN=%RUST_DIR%\target\release\nc-game.exe"
set "DEBUG_BIN=%RUST_DIR%\target\debug\nc-game.exe"

if not exist "%GAME_DIR%" (
    echo nc-game launcher error: game dir not found: %GAME_DIR% 1>&2
    exit /b 66
)

if not exist "%DROPFILE%" (
    echo nc-game launcher error: dropfile not found: %DROPFILE% 1>&2
    exit /b 66
)

if not defined NC_CLIENT_EXPORT_ROOT set "NC_CLIENT_EXPORT_ROOT=%GAME_DIR%\exports"
if not exist "%NC_CLIENT_EXPORT_ROOT%" mkdir "%NC_CLIENT_EXPORT_ROOT%"

if defined NC_CLIENT_QUEUE_DIR (
    if not exist "%NC_CLIENT_QUEUE_DIR%" mkdir "%NC_CLIENT_QUEUE_DIR%"
)

if exist "%RELEASE_BIN%" goto run_release
if exist "%DEBUG_BIN%" goto run_debug
goto run_cargo

:run_release
"%RELEASE_BIN%" --dir "%GAME_DIR%" --dropfile "%DROPFILE%" --encoding cp437 --color-mode ansi16 %EXTRA_ARGS%
exit /b %ERRORLEVEL%

:run_debug
"%DEBUG_BIN%" --dir "%GAME_DIR%" --dropfile "%DROPFILE%" --encoding cp437 --color-mode ansi16 %EXTRA_ARGS%
exit /b %ERRORLEVEL%

:run_cargo
pushd "%RUST_DIR%"
cargo run -q -p nc-game -- --dir "%GAME_DIR%" --dropfile "%DROPFILE%" --encoding cp437 --color-mode ansi16 %EXTRA_ARGS%
set "EXITCODE=%ERRORLEVEL%"
popd
exit /b %EXITCODE%
