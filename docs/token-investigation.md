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
### Goal Achievement: Find the indirect caller of the Token Validation routine
We mapped the call chain backwards from `2000:96c4` (which in DOS memory `2814:96c4` -> `0x31804`).

1. **Return Address Identification**: 
By searching the DOS memory dump `MEMDUMP.BIN`, we located multiple callers pointing to the token validation routine at linear address `0x31804` (invoked as `CALL FAR 3159:0274`).
The exact return address pushed to the stack when checking `setup.tok` was `2895:6B28` (linear `0x2F478`).

2. **Mapping to Ghidra Static Addresses**:
   - `0x2F478` (linear DOS memory) minus the DOSBox PSP load offset (`0x0814` * 16 = `0x8140`) gives `0x27338`.
   - This exactly maps to the static Ghidra address **`2000:7338`**!
   - Thus, the exact indirect caller instruction for `setup.tok` is `CALL FAR` located at **`2000:7333`**!

3. **Reversing the Caller Logic (String Passage)**:
   - The caller pushes parameters by executing:
     ```assembly
     2000:732e: MOV DI,0x6a22
     2000:7331: PUSH CS
     2000:7332: PUSH DI
     2000:7333: CALLF ...
     ```
   - The token string is passed via `CS:DI`! In this case, `CS` in Ghidra is `2000`, so the string is at `2000:6A22`. 
   - We verified that at linear `0x2F372` (which is `0x27372` + `0x8140`) is the Pascal string `\x09setup.tok`!
   
4. **Discovering the full Token Validation Loop**:
   - We also discovered that other `.tok` files (like `Player.Tok`, `Fleets.Tok`, `Database.Tok`) are checked in a massive loop.
   - This master token loop is located at linear `0x31ABC`, which maps to static Ghidra address **`2000:997C`**!
   - This loop invokes the exact same validation function (`0x31804`), but uses a clever compiler optimization to fake a FAR call from a NEAR call within the same segment:
     ```assembly
     MOV DI,0x4e5       ; string offset for Player.Tok
     PUSH CS            ; <--- Fakes the segment part of FAR return address!
     PUSH DI            ; param 1
     PUSH CS            ; param 2
     CALL NEAR ...      ; Pushes IP part of FAR return address
     ```
   - This trick passes the string `CS:DI` and sets up a FAR return stack frame, successfully bypassing standard assumptions and acting as an indirect, disguised invocation of the cross-file integrity check!

## Token Loop and Side Effects Analysis (2000:997C & 2000:96C4)
We statically analyzed the master token loop at `2000:997C` and its core token validation function `2000:96C4`.

1. **`2000:997C` Master Loop**: It sequentially loops through the 8 game tokens (from `Planets.Tok` to `Database.Tok`) and calls the token validation helper `2000:96C4` for each. Crucially, after each call, it simply writes the returned `AL` status into a local stack variable (`MOV byte ptr [BP-1], AL`), overwriting the previous token's result. It does not check the aggregated result or set global flags itself.
2. **`2000:96C4` Token Validator**: If a token is found, it skips the timeout loop and returns `AL=1`. If not found, it enters a polling loop and eventually calls timeout/failure routines (e.g. `3000:4202`, `2000:945b`). This path does not directly set an "integrity bypass" global flag either.
3. **The Starbase 2 Integrity Bypass Flag (`DS:16A4`)**: Disassembling the integrity check routine at `2000:5EE4` revealed that the integrity failure is bypassed if `byte ptr [0x16A4]` is non-zero.
   - Searching the binary for references to `16A4` shows it is *only ever explicitly set to 0* (at `2000:9430`), and is otherwise only compared against `0`. 
   - This suggests `16A4` is set dynamically either via indirect pointers, `REP MOVSB/W` block copies, or initialization from the startup arguments before standard control flow.
4. **Clarification on the Fake FAR Call**: The caller at `2000:7333` executes a standard `CALL FAR 3159:0274`. When adjusted by the DOSBox PSP load offset (`0814`), segment `3159` maps perfectly to `0x31804`, which is precisely the linear address of `2814:96C4` (the token validator `2000:96C4`).
