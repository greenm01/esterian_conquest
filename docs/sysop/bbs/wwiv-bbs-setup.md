# WWIV BBS Setup

WWIV is an experimental BBS host for `nc-door`.

Current posture:

- Windows is the only path documented here
- the basic WWIV chain and dropfile handoff were exercised on a normal user
  install
- remote caller validation is still incomplete on this machine
- do not treat this page as equivalent to the validated Mystic, Synchronet, or
  ENiGMA½ guides yet

## 1. Linux Status

This repo does not yet carry a validated Linux WWIV setup for `nc-door`.

If you are experimenting on Linux later, keep the same core launch shape:

- stage `nc-door`
- pass `--dir`
- pass a real WWIV dropfile such as `CHAIN.TXT`
- keep classic ANSI/CP437 behavior

## 2. Windows Layout

The working layout under a normal user profile was:

```text
C:\Users\<user>\Documents\BBS\wwiv\
  bbs.exe
  wwivd.exe
  data\
  doors\nc-game\
    bin\nc-door.exe
    bin\nc-sysop.exe
    campaign\
```

Keep the BBS root and the staged door files in one stable tree. Do not launch
`nc-door` from a source checkout as the normal sysop path.

## 3. Create the Campaign

Example `config.kdl`:

```kdl
players 4
reservations {
    seat player=1 alias="SYSOP"
}
```

Initialize the BBS campaign:

```text
cd rust
cargo run -q -p nc-sysop -- new-game --bbs C:\Users\<user>\Documents\BBS\wwiv\doors\nc-game\campaign
```

Run yearly maintenance with Task Scheduler or another host-side scheduler:

```text
cd rust
cargo run -q -p nc-sysop -- maint C:\Users\<user>\Documents\BBS\wwiv\doors\nc-game\campaign 1
```

## 4. Chain Shape

WWIV writes `CHAIN.TXT` and can launch an external door command from the BBS
root.

The attempted Windows-native path used:

- `CHAIN.TXT` as the dropfile
- a staged native launcher executable in the WWIV door directory
- a final `nc-door.exe` command that passes `--dir` and `--dropfile`

That keeps the launch shape aligned with the Rust door instead of trying to
wrap `nc-game` or a DOS door.

## 5. Current Windows Blocker

The remaining blocker is the live remote caller path, not campaign creation or
dropfile parsing.

What is already known:

- WWIV can stage `CHAIN.TXT`, `DOOR32.SYS`, and the other node temp files
- the WWIV chain can reach a native launcher
- `nc-door` can start from that launcher against the staged campaign

What is not yet closed:

- a full remote caller session through SyncTERM that cleanly enters the door
  and stays attached to the caller terminal

Until that is closed, treat WWIV as an experimental host target.

## 6. Validation Goal

The success criteria for moving WWIV out of experimental are:

1. connect remotely to WWIV from a normal BBS client
2. log in without using the local node console
3. launch the NC chain
4. confirm the game renders in the caller session
5. verify input, paging, and quit-back-to-BBS behavior

Once that is repeatable, this page can be tightened into a normal validated
setup guide.
