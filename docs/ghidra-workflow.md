# Ghidra Headless Workflow

This project can use Ghidra headlessly on Linux. That avoids any Wayland GUI
compatibility issues and fits the repository's fixture-first reverse-engineering
workflow.

## Install on Linux

Ghidra ships as a zip archive. It does not need a traditional system install.

### 1. Install JDK 21

Ghidra 12.0.2 expects a 64-bit JDK 21.

On Debian or Ubuntu:

```bash
sudo apt update
sudo apt install openjdk-21-jdk unzip
```

Verify:

```bash
java -version
```

Expected major version: `21`

On Arch, CachyOS, or another Arch-derived distribution:

```bash
sudo pacman -Syu ghidra jdk21-openjdk
```

That typically installs Ghidra under:

```text
/usr/share/ghidra
```

### 2. Download and unpack Ghidra

Pick a local tools directory. Example:

```bash
mkdir -p "$HOME/tools"
cd "$HOME/tools"
wget https://github.com/NationalSecurityAgency/ghidra/releases/download/Ghidra_12.0.2_build/ghidra_12.0.2_PUBLIC_20260129.zip
unzip ghidra_12.0.2_PUBLIC_20260129.zip
```

That creates a directory like:

```text
$HOME/tools/ghidra_12.0.2_PUBLIC
```

### 3. Export `GHIDRA_HOME`

Add this to your shell profile:

```bash
export GHIDRA_HOME="$HOME/tools/ghidra_12.0.2_PUBLIC"
```

Reload your shell or run the export in the current terminal.

If Ghidra came from the Arch/CachyOS package, this is usually:

```bash
export GHIDRA_HOME=/usr/share/ghidra
```

### 4. Verify headless mode

```bash
"$GHIDRA_HOME"/support/analyzeHeadless
```

It should print usage text.

## Wayland Notes

Headless analysis does not need X11, XWayland, or a running desktop session.

If you later want the GUI, `ghidraRun` may still work under Wayland depending on
your desktop, Java runtime, and XWayland setup, but the workflow in this repo
does not depend on that.

## Repo Usage

Use the wrapper script:

```bash
tools/ghidra_ecmaint.sh
```

Default behavior:

- imports `original/v1.5/ECMAINT.EXE`
- creates a Ghidra project under `.ghidra/projects`
- writes logs under `artifacts/ghidra/ecmaint`
- reuses the project on later runs
- auto-detects packaged installs at `/usr/share/ghidra`
- stores Ghidra config/cache under repo-local `.ghidra/` paths

Generated repo-local state:

- `.ghidra/projects/ec-v15.gpr`
- `.ghidra/projects/ec-v15.rep/`
- `artifacts/ghidra/ecmaint/analyze.log`
- `artifacts/ghidra/ecmaint/script.log`

### Common commands

Analyze the canonical original binary:

```bash
tools/ghidra_ecmaint.sh
```

Analyze a fixture-local copy instead:

```bash
tools/ghidra_ecmaint.sh fixtures/ecmaint-starbase-pre/v1.5/ECMAINT.EXE
```

Force re-import and overwrite the existing program:

```bash
tools/ghidra_ecmaint.sh --overwrite
```

Apply a per-file timeout only when you want one:

```bash
tools/ghidra_ecmaint.sh --analysis-timeout 600
```

Use a different project name:

```bash
tools/ghidra_ecmaint.sh --project ec-v15-lab
```

## First Analysis Pass

From the repo root:

```bash
tools/ghidra_ecmaint.sh --overwrite
```

Expected high-signal log lines:

- `Using Loader: Old-style DOS Executable (MZ)`
- `Using Language/Compiler: x86:LE:16:Real Mode:default`
- `REPORT: Analysis succeeded`

The current known-good baseline for `original/v1.5/ECMAINT.EXE` is:

- loader: old-style DOS MZ executable
- language: 16-bit x86 real mode
- MD5: `21489ef9798df77b20b7a02eb9347071`

Review the analysis log with:

```bash
tail -n 40 artifacts/ghidra/ecmaint/analyze.log
```

If you want to start fresh, remove the project files and rerun:

```bash
rm -rf .ghidra/projects/ec-v15.gpr .ghidra/projects/ec-v15.rep
tools/ghidra_ecmaint.sh --overwrite
```

## Recommended First Pass

For the current `ECMAINT` work, the first headless pass should answer:

1. where `BASES.DAT`, `PLAYER.DAT`, and `FLEETS.DAT` are opened
2. which code paths reference the starbase-related offsets already recovered
3. whether there is a second table or record family for multi-starbase support

Static analysis should stay subordinate to fixture evidence:

- use disassembly to generate hypotheses
- validate them with controlled pre/post `.DAT` diffs
- only promote semantics after repeated confirmation

## Suggested Analysis Routine

Use this sequence for `ECMAINT` work:

1. preserve a controlled pre/post fixture pair with the existing black-box workflow
2. rerun `tools/ghidra_ecmaint.sh --overwrite` if the binary or project changed
3. inspect strings, entry points, and file access sites in Ghidra
4. trace the code paths that touch the fixture offsets already confirmed in docs
5. turn any promising hypothesis back into a new controlled fixture experiment

This keeps disassembly grounded in observed engine behavior instead of drifting
into speculative structure naming.
