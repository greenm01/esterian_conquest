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

Apply the live-dump integrity labels to the existing `ecmaint-live` project:

```bash
XDG_CONFIG_HOME="$PWD/.ghidra/xdg-config" \
XDG_CACHE_HOME="$PWD/.ghidra/xdg-cache" \
JAVA_HOME=/usr/lib/jvm/java-21-openjdk \
"$HOME/tools/ghidra_12.0.2_PUBLIC/support/analyzeHeadless" \
  "$PWD/.ghidra/projects" ecmaint-live \
  -process MEMDUMP.BIN \
  -scriptPath "$PWD/tools/ghidra_scripts" \
  -postScript ECMaintNameIntegrityAnchors.java "$PWD/artifacts/ghidra/ecmaint-live" \
  -noanalysis
```

This saves the labels/function names in the project and refreshes:

- `artifacts/ghidra/ecmaint-live/integrity-anchors.txt`
- `artifacts/ghidra/ecmaint-live/functions.txt` after rerunning `ECMaintDumpAnchors.java`

Apply the token-anchor labels/report to the same live-dump project:

```bash
XDG_CONFIG_HOME="$PWD/.ghidra/xdg-config" \
XDG_CACHE_HOME="$PWD/.ghidra/xdg-cache" \
JAVA_HOME=/usr/lib/jvm/java-21-openjdk \
"$HOME/tools/ghidra_12.0.2_PUBLIC/support/analyzeHeadless" \
  "$PWD/.ghidra/projects" ecmaint-live \
  -process MEMDUMP.BIN \
  -scriptPath "$PWD/tools/ghidra_scripts" \
  -postScript ECMaintTokenAnchors.java "$PWD/artifacts/ghidra/ecmaint-live" \
  -noanalysis
```

This saves:

- token cluster labels at `2000:841b`, `2000:6fc6`, and `2000:9680`
- function name `ecmaint_check_named_token_candidate` at `2000:96c4`
- function name `ecmaint_move_tok_delete_wrapper_candidate` at `2000:9cb0`
- function name `ecmaint_token_wait_timeout_helper` at `2000:9b13`
- label `ecmaint_move_tok_recovery_candidate` at `2000:9d48`
- report `artifacts/ghidra/ecmaint-live/token-anchors.txt`

Generate the deeper token reachability report from the same project:

```bash
XDG_CONFIG_HOME="$PWD/.ghidra/xdg-config" \
XDG_CACHE_HOME="$PWD/.ghidra/xdg-cache" \
JAVA_HOME=/usr/lib/jvm/java-21-openjdk \
"$HOME/tools/ghidra_12.0.2_PUBLIC/support/analyzeHeadless" \
  "$PWD/.ghidra/projects" ecmaint-live \
  -process MEMDUMP.BIN \
  -scriptPath "$PWD/tools/ghidra_scripts" \
  -postScript ECMaintTokenReachabilityReport.java "$PWD/artifacts/ghidra/ecmaint-live" \
  -noanalysis
```

This refreshes:

- `artifacts/ghidra/ecmaint-live/token-reachability.txt`
- the tiny `Move.Tok` wrapper/callee ranges at `2000:9c91`, `2000:9cb0`,
  `2000:96c4`, and labeled start `2000:9887`
- the `Move.Tok` recovery block report at `2000:9d48..0x9e1d`
- the timestamp-message helper range at `2000:945b..0x967c`
- the note that DS:`46cc` / CS:`0653` are runtime segment-relative operands,
  not reliable raw-import linear string addresses

Generate the stack-derived token caller report from the same project:

```bash
XDG_CONFIG_HOME="$PWD/.ghidra/xdg-config" \
XDG_CACHE_HOME="$PWD/.ghidra/xdg-cache" \
JAVA_HOME=/usr/lib/jvm/java-21-openjdk \
"$HOME/tools/ghidra_12.0.2_PUBLIC/support/analyzeHeadless" \
  "$PWD/.ghidra/projects" ecmaint-live \
  -process MEMDUMP.BIN \
  -scriptPath "$PWD/tools/ghidra_scripts" \
  -postScript ECMaintTokenCallerReport.java "$PWD/artifacts/ghidra/ecmaint-live" \
  -noanalysis
```

This refreshes:

- `artifacts/ghidra/ecmaint-live/token-callers.txt`
- the dynamic-stack-derived lead from the first clean `2000:96c4` hit
- the raw-import caller vicinity at `2000:731f..7338`
- the current negative result that there are still no direct static xrefs to
  `2000:9d48` or `2000:9e1e` in the saved disassembly

Generate the focused `2000:5EE4` / `IPBM.DAT` branch report:

```bash
tools/run_ghidra_script_args.sh ecmaint-live Report5EE4IPBM.java
```

This refreshes:

- `artifacts/ghidra/ecmaint-live/5ee4-ipbm.txt`
- the player-indexed `IPBM.DAT` branch at `2000:675A..68E8`
- the follow-on `IPBM` summary branch at `2000:68E9..69B8`
- the current DS:`31F8` / DS:`3538` / `0x2F72` / `0x2F76` data-flow notes

Generate the whole-program scalar sweep for the `IPBM` scratch block:

```bash
tools/run_ghidra_script_args.sh ecmaint-live ReportIPBMScratchScalarUses.java
```

This refreshes:

- `artifacts/ghidra/ecmaint-live/ipbm-scratch-uses.txt`
- the current uses of `3538..3553` across the live image
- the producer/consumer lead at function `0000:02c0`

Generate the full function dump for the summary dispatcher at `0000:02c0`:

```bash
tools/run_ghidra_script_args.sh ecmaint-live ReportIPBMScratchFunction.java
```

This refreshes:

- `artifacts/ghidra/ecmaint-live/ipbm-scratch-function.txt`
- the summary-kind dispatch at `0000:02c0`
- the current kind-`1` / kind-`2` / kind-`3` scratch-block split

Generate the helper/tail correction reports for the kind-`3` path:

```bash
tools/run_ghidra_script_args.sh ecmaint-live ReportIPBMNormalizer.java
tools/run_ghidra_script_args.sh ecmaint-live ReportSummaryHelperRegion.java
```

This refreshes:

- `artifacts/ghidra/ecmaint-live/ipbm-normalizer.txt`
- `artifacts/ghidra/ecmaint-live/summary-helper-region.txt`
- the correction that `2000:c0cd` is only a tiny copy-tail / mis-carved entry
- the nearby clean bounded-copy helper at `2000:c0dc`

Generate the focused shared post-kind pipeline report:

```bash
tools/run_ghidra_script_args.sh ecmaint-live ReportIPBMPostKindPipeline.java
```

This refreshes:

- `artifacts/ghidra/ecmaint-live/ipbm-postkind-pipeline.txt`
- the common `0000:07da..0ea6` comparison / merge stage inside `0000:02c0`
- the writeback from canonicalized tuple state back into summary offsets
  `+0x01`, `+0x02`, and `+0x05`

Generate the tail transition report for the kind split at the end of the
post-kind pipeline:

```bash
tools/run_ghidra_script_args.sh ecmaint-live ReportIPBMTailTransition.java
```

This refreshes:

- `artifacts/ghidra/ecmaint-live/ipbm-tail-transition.txt`
- the kind-2 side path through `0x2000:c100`
- the confirmed kind-3 tuple writeback:
  - tuple A -> `3541`, `3543..3547`
  - tuple B -> `3542`, `3549..354d`
  - tuple C -> `354f..3553`

Generate the focused kind-2 matcher report for the remaining fleet/base
pairing keys:

```bash
tools/run_ghidra_script_args.sh ecmaint-live ReportKind2Matcher.java
```

This refreshes:

- `artifacts/ghidra/ecmaint-live/kind2-matcher.txt`
- the concrete pairing loop at `0000:03DF..06AE`
- the decode path from base-side summary `+0x06` through helper `0x2000:c09a`
  into scratch rooted at `3558`
- the structural accept path that decodes candidate kind-1 summary `+0x06`
  through helper `0x2000:c067` and compares against `[0x355A]`

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
