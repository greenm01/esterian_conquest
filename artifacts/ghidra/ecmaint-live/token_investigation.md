# Token Investigation Notes

## Token File List Discovery
Found a block of token filenames stored contiguously in memory starting around file offset `0x2991F` (which maps to Ghidra address `2000:991F`).
The list includes:
- `Planets.Tok`
- `Fleets.Tok`
- `Player.Tok`
- `IPBMs.Tok`
- `Conquest.Tok`
- `Message.Tok`
- `Results.Tok`
- `Database.Tok`

Immediately following this string block, at file offset `0x2997E` (Ghidra `2000:997E`), there is a function prologue (`PUSH BP`, `MOV BP, SP`, `SUB SP, 2`) followed by a series of calls that appear to iterate over these token strings:
```assembly
MOV DI, 04ce
PUSH CS
PUSH DI
PUSH CS
CALL ...
MOV [BP-1], AL
```
This is highly likely to be the core token checking/validation function that probes for the existence or state of these various token files.

## Addressing Scheme Validation
- Ghidra `ecmaint-live` project maps `MEMDUMP.BIN` starting at `0000:0000`.
- Segment `2000:` in Ghidra corresponds exactly to file offset `0x20000` in `MEMDUMP.BIN`.
- Live debugger PSP `0814` maps to segment `2814:` dynamically.

Next step: Disassemble and analyze the function at `2000:997e`.CS for main token check is 0x2895
Found core startup sequence!
Checking global variables in 5ee4
